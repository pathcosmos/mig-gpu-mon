use anyhow::{Context, Result};
use nvml_wrapper::bitmasks::device::ThrottleReasons;
use nvml_wrapper::enum_wrappers::device::{
    Clock, EccCounter, MemoryError, PcieUtilCounter, PerformanceState, TemperatureSensor,
    TemperatureThreshold,
};
use nvml_wrapper::enums::device::{DeviceArchitecture, UsedGpuMemory};
use nvml_wrapper::Device;
use nvml_wrapper::Nvml;
use nvml_wrapper_sys::bindings::{
    nvmlDeviceAttributes_t, nvmlDevice_t, nvmlGpmMetricId_t_NVML_GPM_METRIC_DRAM_BW_UTIL,
    nvmlGpmMetricsGet_t, nvmlGpmSample_t, nvmlGpmSupport_t, nvmlProcessUtilizationSample_t,
    nvmlSample_t, NvmlLib, NVML_GPM_METRICS_GET_VERSION, NVML_GPM_SUPPORT_VERSION,
};
use std::cell::RefCell;
use std::collections::HashMap;
use std::ffi::OsStr;
use std::os::raw::c_uint;

use super::metrics::{GpuMetrics, GpuProcessInfo};

/// Cached static device info that never changes at runtime
#[derive(Clone)]
struct DeviceInfo {
    name: String,
    uuid: String,
    architecture: Option<&'static str>,
    compute_capability: Option<String>,
    ecc_enabled: Option<bool>,
    temp_shutdown: Option<u32>,
    temp_slowdown: Option<u32>,
    /// MIG GPU instance slice count (e.g. 3 for 3g.40gb) — only set on MIG devices
    gpu_instance_slice_count: Option<u32>,
    /// Whether the device supports GPM (GPU Performance Monitoring) — Hopper+ only
    gpm_supported: bool,
    /// MIG GPU instance ID (for GPM MIG sampling via parent device)
    gpu_instance_id: Option<u32>,
}

pub struct NvmlCollector {
    nvml: Nvml,
    raw_lib: NvmlLib,
    /// Cache: device handle pointer → static info.
    /// RefCell for interior mutability — cache is populated lazily while
    /// NVML device handles borrow &self.nvml.
    device_cache: RefCell<HashMap<usize, DeviceInfo>>,
    /// Reusable buffer for process utilization samples (avoid alloc per tick)
    proc_sample_buf: RefCell<Vec<nvmlProcessUtilizationSample_t>>,
    /// Reusable buffer for GPU utilization samples (avoid alloc per tick)
    sample_buf: RefCell<Vec<nvmlSample_t>>,
    /// Previous GPM samples for DRAM BW util computation (keyed by device handle pointer).
    /// Each value is an allocated nvmlGpmSample_t that must be freed on drop.
    gpm_prev_samples: RefCell<HashMap<usize, nvmlGpmSample_t>>,
}

impl NvmlCollector {
    /// Well-known paths where libnvidia-ml.so may be found on Linux systems.
    /// Order: dynamic linker names first, then distro-specific, container, cloud, WSL.
    #[cfg(target_os = "linux")]
    const NVML_LIB_PATHS: &[&str] = &[
        // Dynamic linker — works when LD_LIBRARY_PATH or ldconfig is configured
        "libnvidia-ml.so.1",
        "libnvidia-ml.so",
        // Debian / Ubuntu (x86_64)
        "/usr/lib/x86_64-linux-gnu/libnvidia-ml.so.1",
        // RHEL / CentOS / Rocky / Amazon Linux
        "/usr/lib64/libnvidia-ml.so.1",
        // Debian / Ubuntu (ARM64 — e.g. AWS Graviton GPU instances)
        "/usr/lib/aarch64-linux-gnu/libnvidia-ml.so.1",
        // NVIDIA Container Toolkit (vast.io, RunPod, AWS EKS/ECS, GCP GKE, Azure AKS)
        "/usr/local/nvidia/lib64/libnvidia-ml.so.1",
        "/usr/local/nvidia/lib/libnvidia-ml.so.1",
        // NVIDIA GPU Operator / driver container (Kubernetes)
        "/run/nvidia/driver/usr/lib/x86_64-linux-gnu/libnvidia-ml.so.1",
        "/run/nvidia/driver/usr/lib64/libnvidia-ml.so.1",
        // CUDA stubs (build-only — no runtime metrics, but allows init check)
        "/usr/local/cuda/lib64/stubs/libnvidia-ml.so",
        // WSL2
        "/usr/lib/wsl/lib/libnvidia-ml.so.1",
    ];

    /// Build the list of candidate library paths: user override + LD_LIBRARY_PATH + built-in.
    #[cfg(target_os = "linux")]
    fn lib_search_paths(user_path: Option<&str>) -> Vec<String> {
        let mut paths: Vec<String> = Vec::with_capacity(16);

        // 1. User-specified path takes highest priority
        if let Some(p) = user_path {
            paths.push(p.to_string());
        }

        // 2. Paths derived from LD_LIBRARY_PATH (may contain cloud-specific entries)
        if let Ok(ld_path) = std::env::var("LD_LIBRARY_PATH") {
            for dir in ld_path.split(':').filter(|s| !s.is_empty()) {
                let candidate = format!("{dir}/libnvidia-ml.so.1");
                if !paths.contains(&candidate) {
                    paths.push(candidate);
                }
            }
        }

        // 3. Built-in well-known paths
        for &p in Self::NVML_LIB_PATHS {
            let s = p.to_string();
            if !paths.contains(&s) {
                paths.push(s);
            }
        }

        paths
    }

    pub fn new(nvml_lib_path: Option<&str>) -> Result<Self> {
        let nvml = Self::init_nvml(nvml_lib_path)
            .context("Failed to initialize NVML. Is the NVIDIA driver installed?")?;

        let raw_lib = unsafe {
            #[cfg(target_os = "linux")]
            let lib =
                Self::load_raw_lib(nvml_lib_path).context("Failed to load NVML shared library")?;
            #[cfg(target_os = "windows")]
            let lib = NvmlLib::new("nvml.dll").context("Failed to load NVML DLL")?;
            lib
        };

        Ok(Self {
            nvml,
            raw_lib,
            device_cache: RefCell::new(HashMap::new()),
            proc_sample_buf: RefCell::new(Vec::with_capacity(32)),
            sample_buf: RefCell::new(Vec::with_capacity(128)),
            gpm_prev_samples: RefCell::new(HashMap::new()),
        })
    }

    fn init_nvml(user_path: Option<&str>) -> Result<Nvml, nvml_wrapper::error::NvmlError> {
        #[cfg(target_os = "linux")]
        {
            for path in Self::lib_search_paths(user_path) {
                if let Ok(nvml) = Nvml::builder().lib_path(OsStr::new(&path)).init() {
                    return Ok(nvml);
                }
            }
        }
        #[cfg(not(target_os = "linux"))]
        {
            let _ = user_path;
        }
        // Final attempt — default search
        Nvml::init()
    }

    #[cfg(target_os = "linux")]
    unsafe fn load_raw_lib(user_path: Option<&str>) -> Result<NvmlLib> {
        for path in Self::lib_search_paths(user_path) {
            if let Ok(lib) = NvmlLib::new(&path) {
                return Ok(lib);
            }
        }
        NvmlLib::new("libnvidia-ml.so.1").map_err(|e| anyhow::anyhow!("{e}"))
    }

    /// Collect metrics from all GPUs and MIG instances
    pub fn collect_all(&self) -> Result<Vec<GpuMetrics>> {
        let device_count = self.nvml.device_count()?;
        let mut all_metrics = Vec::with_capacity(device_count as usize * 2);

        for i in 0..device_count {
            let device = self.nvml.device_by_index(i)?;

            if self.is_mig_enabled(&device) {
                if let Ok(mig_metrics) = self.collect_mig_instances(&device, i) {
                    all_metrics.extend(mig_metrics);
                }
            }

            if let Ok(metrics) = self.collect_device_metrics(&device, i, false, None) {
                all_metrics.push(metrics);
            }
        }

        Ok(all_metrics)
    }

    fn is_mig_enabled(&self, device: &Device) -> bool {
        unsafe {
            let handle = device.handle();
            let mut current_mode: c_uint = 0;
            let mut pending_mode: c_uint = 0;
            let ret =
                self.raw_lib
                    .nvmlDeviceGetMigMode(handle, &mut current_mode, &mut pending_mode);
            ret == 0 && current_mode == 1
        }
    }

    fn collect_mig_instances(
        &self,
        parent_device: &Device,
        gpu_index: u32,
    ) -> Result<Vec<GpuMetrics>> {
        let mut mig_metrics = Vec::new();

        unsafe {
            let parent_handle = parent_device.handle();

            let mut max_count: c_uint = 0;
            let ret = self
                .raw_lib
                .nvmlDeviceGetMaxMigDeviceCount(parent_handle, &mut max_count);
            if ret != 0 {
                return Ok(mig_metrics);
            }

            // Pre-fetch parent-level GPU util via samples (for slice-ratio fallback)
            let parent_sampled_util = self.get_gpu_util_from_samples(parent_handle);
            // MaxMigDeviceCount = total number of GPU instance slices (e.g. 7 for H100)
            let total_slices = max_count;

            // GPM support must be checked on the parent device (not MIG handles)
            let parent_info = self.get_device_info(parent_device, false);
            let parent_gpm_supported = parent_info.gpm_supported;

            // === Phase 1: Collect base metrics (VRAM, utilization) for all MIG instances ===
            // All VRAM queries happen here, BEFORE any GPM calls that could corrupt NVML state.
            // Also collect MIG handles for phase 2.
            let mut phase1: Vec<(nvmlDevice_t, GpuMetrics)> = Vec::new();

            for mig_idx in 0..max_count {
                let mut mig_handle: nvmlDevice_t = std::ptr::null_mut();
                let ret = self.raw_lib.nvmlDeviceGetMigDeviceHandleByIndex(
                    parent_handle,
                    mig_idx,
                    &mut mig_handle,
                );
                if ret != 0 || mig_handle.is_null() {
                    continue;
                }

                let mig_device = Device::new(mig_handle, &self.nvml);

                if let Ok(mut metrics) =
                    self.collect_device_metrics(&mig_device, mig_idx, true, Some(gpu_index))
                {
                    // Fallback 1: process utilization (no GPM involvement)
                    if metrics.gpu_util.is_none() {
                        let (sm, mem) = self.get_process_utilization(mig_handle);
                        if sm > 0 {
                            metrics.gpu_util = Some(sm);
                            metrics.sm_util = Some(sm);
                        }
                        if mem > 0 {
                            metrics.memory_util = Some(mem);
                        }
                    }

                    // Fallback 2: scale parent's sampled util by MIG slice ratio (no GPM)
                    if metrics.gpu_util.is_none() {
                        if let Some(p_util) = parent_sampled_util {
                            let mig_info = self.get_device_info(&mig_device, true);
                            if let Some(mig_slices) = mig_info.gpu_instance_slice_count {
                                let scaled = ((p_util as u64) * (total_slices as u64)
                                    / (mig_slices as u64))
                                    .min(100) as u32;
                                metrics.gpu_util = Some(scaled);
                                metrics.sm_util = Some(scaled);
                            }
                        }
                    }

                    metrics.name = format!("MIG {mig_idx} (GPU {gpu_index}: {})", metrics.name);
                    phase1.push((mig_handle, metrics));
                }
            }

            // === Phase 2: GPM fallbacks (after all VRAM queries are done) ===
            // GPM calls (nvmlGpmMigSampleGet) may corrupt NVML state, but all VRAM
            // has already been collected in phase 1, so corruption won't affect it.
            for (mig_handle, metrics) in &mut phase1 {
                if metrics.memory_util.is_none() && parent_gpm_supported {
                    let mig_device = Device::new(*mig_handle, &self.nvml);
                    let mig_info = self.get_device_info(&mig_device, true);
                    metrics.memory_util = self.get_dram_bw_util_gpm(
                        *mig_handle,
                        true,
                        mig_info.gpu_instance_id,
                        Some(parent_handle),
                    );
                }
            }

            mig_metrics.extend(phase1.into_iter().map(|(_, m)| m));
        }

        Ok(mig_metrics)
    }

    /// Get aggregated process utilization via raw NVML API.
    /// Reuses internal buffer to avoid allocation per call.
    unsafe fn get_process_utilization(&self, device_handle: nvmlDevice_t) -> (u32, u32) {
        let mut count: c_uint = 0;
        let ret = self.raw_lib.nvmlDeviceGetProcessUtilization(
            device_handle,
            std::ptr::null_mut(),
            &mut count,
            0,
        );

        if ret != 7 || count == 0 {
            return (0, 0);
        }

        let mut buf = self.proc_sample_buf.borrow_mut();
        let needed = count as usize;
        if buf.len() < needed {
            buf.resize(needed, std::mem::zeroed());
        }

        let ret = self.raw_lib.nvmlDeviceGetProcessUtilization(
            device_handle,
            buf.as_mut_ptr(),
            &mut count,
            0,
        );

        if ret != 0 {
            return (0, 0);
        }

        // Shrink buffer if it grew much larger than needed (prevent unbounded growth)
        let actual = count as usize;
        if buf.capacity() > actual.max(32) * 4 {
            buf.truncate(actual);
            buf.shrink_to(actual * 2);
        }

        let mut max_sm: u32 = 0;
        let mut max_mem: u32 = 0;
        for sample in &buf[..actual] {
            if sample.smUtil <= 100 {
                max_sm = max_sm.max(sample.smUtil);
            }
            if sample.memUtil <= 100 {
                max_mem = max_mem.max(sample.memUtil);
            }
        }

        (max_sm, max_mem)
    }

    /// Get GPU utilization from nvmlDeviceGetSamples (parent device only).
    /// Returns the average of recent samples scaled to 0-100%.
    /// Reuses internal buffer to avoid allocation per call.
    unsafe fn get_gpu_util_from_samples(&self, device_handle: nvmlDevice_t) -> Option<u32> {
        let mut val_type: c_uint = 0;
        let mut count: c_uint = 0;

        // Size query
        let ret = self.raw_lib.nvmlDeviceGetSamples(
            device_handle,
            0, // NVML_GPU_UTILIZATION_SAMPLES
            0,
            &mut val_type,
            &mut count,
            std::ptr::null_mut(),
        );

        // ret==0 means count is already set; ret==7 means INSUFFICIENT_SIZE (also sets count)
        if (ret != 0 && ret != 7) || count == 0 {
            return None;
        }

        let mut buf = self.sample_buf.borrow_mut();
        let needed = count as usize;
        if buf.len() < needed {
            buf.resize(needed, std::mem::zeroed());
        }

        let ret = self.raw_lib.nvmlDeviceGetSamples(
            device_handle,
            0,
            0,
            &mut val_type,
            &mut count,
            buf.as_mut_ptr(),
        );
        if ret != 0 || count == 0 {
            return None;
        }

        // Shrink buffer if it grew much larger than needed (prevent unbounded growth)
        let actual = count as usize;
        if buf.capacity() > actual.max(128) * 4 {
            buf.truncate(actual);
            buf.shrink_to(actual * 2);
        }

        // Use last ~5 samples for a responsive average (not all 120)
        let n = actual.min(5);
        let start = actual - n;
        let sum: u64 = buf[start..actual]
            .iter()
            .map(|s| s.sampleValue.uiVal as u64)
            .sum();
        let avg = sum / n as u64;

        // Driver 535 returns raw values ~290000 range; /10000 gives 0-100%
        let util = (avg / 10000).min(100) as u32;
        Some(util)
    }

    /// Get DRAM bandwidth utilization via GPM API (Hopper+ only).
    /// Uses previous tick's sample and current sample to compute utilization.
    /// For MIG: uses nvmlGpmMigSampleGet with gpuInstanceId on parent handle.
    /// For regular GPU: uses nvmlGpmSampleGet on device handle.
    unsafe fn get_dram_bw_util_gpm(
        &self,
        device_handle: nvmlDevice_t,
        is_mig: bool,
        gpu_instance_id: Option<u32>,
        parent_handle: Option<nvmlDevice_t>,
    ) -> Option<u32> {
        // Allocate new sample
        let mut new_sample: nvmlGpmSample_t = std::ptr::null_mut();
        if self.raw_lib.nvmlGpmSampleAlloc(&mut new_sample) != 0 || new_sample.is_null() {
            return None;
        }

        // Take sample
        let ret = if is_mig {
            let parent = parent_handle?;
            let gi_id = gpu_instance_id?;
            self.raw_lib.nvmlGpmMigSampleGet(parent, gi_id, new_sample)
        } else {
            self.raw_lib.nvmlGpmSampleGet(device_handle, new_sample)
        };

        if ret != 0 {
            self.raw_lib.nvmlGpmSampleFree(new_sample);
            return None;
        }

        let key = device_handle as usize;
        let mut prev_map = self.gpm_prev_samples.borrow_mut();

        let result = if let Some(&prev_sample) = prev_map.get(&key) {
            // Compute metrics from previous + current samples
            let mut metrics_get: nvmlGpmMetricsGet_t = std::mem::zeroed();
            metrics_get.version = NVML_GPM_METRICS_GET_VERSION;
            metrics_get.numMetrics = 1;
            metrics_get.sample1 = prev_sample;
            metrics_get.sample2 = new_sample;
            metrics_get.metrics[0].metricId = nvmlGpmMetricId_t_NVML_GPM_METRIC_DRAM_BW_UTIL;

            if self.raw_lib.nvmlGpmMetricsGet(&mut metrics_get) == 0
                && metrics_get.metrics[0].nvmlReturn == 0
            {
                let val = metrics_get.metrics[0].value;
                Some((val.round() as u32).min(100))
            } else {
                None
            }
        } else {
            // First tick — no previous sample yet, just store for next tick
            None
        };

        // Free old sample if exists, store new one
        if let Some(old) = prev_map.insert(key, new_sample) {
            self.raw_lib.nvmlGpmSampleFree(old);
        }

        result
    }

    /// Get cached or fetch device info. Static fields never change at runtime.
    /// `skip_gpm_query`: set true for MIG handles — nvmlGpmQueryDeviceSupport on MIG handles
    /// can corrupt NVML driver state, causing subsequent memory_info() calls to fail.
    /// GPM support should be checked on the parent device instead.
    fn get_device_info(&self, device: &Device, skip_gpm_query: bool) -> DeviceInfo {
        let key = unsafe { device.handle() } as usize;
        let cache = self.device_cache.borrow();
        if let Some(info) = cache.get(&key) {
            return info.clone();
        }
        drop(cache);

        let info = DeviceInfo {
            name: device.name().unwrap_or_else(|_| "Unknown GPU".to_string()),
            uuid: device.uuid().unwrap_or_else(|_| "N/A".to_string()),
            architecture: device.architecture().ok().map(format_architecture),
            compute_capability: device
                .cuda_compute_capability()
                .ok()
                .map(|cc| format!("{}.{}", cc.major, cc.minor)),
            ecc_enabled: device.is_ecc_enabled().ok().map(|e| e.currently_enabled),
            temp_shutdown: device
                .temperature_threshold(TemperatureThreshold::Shutdown)
                .ok(),
            temp_slowdown: device
                .temperature_threshold(TemperatureThreshold::Slowdown)
                .ok(),
            gpu_instance_slice_count: unsafe {
                let mut attrs: nvmlDeviceAttributes_t = std::mem::zeroed();
                if self
                    .raw_lib
                    .nvmlDeviceGetAttributes_v2(device.handle(), &mut attrs)
                    == 0
                    && attrs.gpuInstanceSliceCount > 0
                {
                    Some(attrs.gpuInstanceSliceCount)
                } else {
                    None
                }
            },
            // Never query GPM support on MIG handles — it corrupts NVML state.
            // GPM support is checked on the parent device in collect_mig_instances.
            gpm_supported: if skip_gpm_query {
                false
            } else {
                unsafe {
                    let mut support: nvmlGpmSupport_t = std::mem::zeroed();
                    support.version = NVML_GPM_SUPPORT_VERSION;
                    if self
                        .raw_lib
                        .nvmlGpmQueryDeviceSupport(device.handle(), &mut support)
                        == 0
                    {
                        support.isSupportedDevice != 0
                    } else {
                        false
                    }
                }
            },
            gpu_instance_id: unsafe {
                let mut id: c_uint = 0;
                if self
                    .raw_lib
                    .nvmlDeviceGetGpuInstanceId(device.handle(), &mut id)
                    == 0
                {
                    Some(id)
                } else {
                    None
                }
            },
        };

        self.device_cache.borrow_mut().insert(key, info.clone());
        info
    }

    fn collect_device_metrics(
        &self,
        device: &Device,
        index: u32,
        is_mig: bool,
        parent_index: Option<u32>,
    ) -> Result<GpuMetrics> {
        // For MIG handles, skip GPM support query in get_device_info — it corrupts NVML state.
        let info = self.get_device_info(device, is_mig);

        // VRAM query first — before any GPM calls that might disturb NVML state on MIG handles
        let (memory_used, memory_total) = device
            .memory_info()
            .map(|m| (Some(m.used), Some(m.total)))
            .unwrap_or((None, None));

        let (gpu_util, memory_util): (Option<u32>, Option<u32>) = match device.utilization_rates() {
            Ok(u) => (Some(u.gpu), Some(u.memory)),
            Err(_) => {
                // Fallback: try nvmlDeviceGetSamples on non-MIG (parent) devices
                let sampled = if !is_mig {
                    unsafe { self.get_gpu_util_from_samples(device.handle()) }
                } else {
                    None
                };
                (sampled, None) // memory_util: GPM fallback attempted in caller for MIG
            }
        };

        // GPM fallback for memory_util on non-MIG devices only (Hopper+ only).
        // MIG devices must use nvmlGpmMigSampleGet via parent — handled in collect_mig_instances.
        let memory_util = if memory_util.is_none() && !is_mig && info.gpm_supported {
            unsafe { self.get_dram_bw_util_gpm(device.handle(), false, None, None) }
        } else {
            memory_util
        };

        let temperature = device.temperature(TemperatureSensor::Gpu).ok();
        let power_usage = device.power_usage().ok();
        let power_limit = device.power_management_limit().ok();

        // Extended dynamic metrics — all .ok() wrapped for graceful MIG/vGPU fallback
        let clock_graphics_mhz = device.clock_info(Clock::Graphics).ok();
        let clock_sm_mhz = device.clock_info(Clock::SM).ok();
        let clock_memory_mhz = device.clock_info(Clock::Memory).ok();
        let pcie_tx_kbps = device.pcie_throughput(PcieUtilCounter::Send).ok();
        let pcie_rx_kbps = device.pcie_throughput(PcieUtilCounter::Receive).ok();
        let pcie_gen = device.current_pcie_link_gen().ok();
        let pcie_width = device.current_pcie_link_width().ok();
        let performance_state = device.performance_state().ok().map(format_pstate);
        let throttle_reasons = device
            .current_throttle_reasons()
            .ok()
            .map(format_throttle_reasons);
        let encoder_util = device.encoder_utilization().ok().map(|u| u.utilization);
        let decoder_util = device.decoder_utilization().ok().map(|u| u.utilization);
        let ecc_errors_corrected = device
            .total_ecc_errors(MemoryError::Corrected, EccCounter::Volatile)
            .ok();
        let ecc_errors_uncorrected = device
            .total_ecc_errors(MemoryError::Uncorrected, EccCounter::Volatile)
            .ok();

        let (process_count, top_processes) = match device.running_compute_processes() {
            Ok(procs) => {
                let count = procs.len() as u32;
                let mut proc_infos: Vec<GpuProcessInfo> = procs
                    .iter()
                    .filter_map(|p| {
                        let vram = match p.used_gpu_memory {
                            UsedGpuMemory::Used(bytes) => bytes,
                            UsedGpuMemory::Unavailable => return None,
                        };
                        Some(GpuProcessInfo {
                            pid: p.pid,
                            name: Self::process_name(p.pid),
                            vram_used: vram,
                        })
                    })
                    .collect();
                // Partial sort optimization: O(n) select for top-5 instead of O(n log n) full sort
                if proc_infos.len() > 5 {
                    proc_infos.select_nth_unstable_by(4, |a, b| b.vram_used.cmp(&a.vram_used));
                    proc_infos.truncate(5);
                    proc_infos.sort_by(|a, b| b.vram_used.cmp(&a.vram_used));
                } else {
                    proc_infos.sort_by(|a, b| b.vram_used.cmp(&a.vram_used));
                }
                (count, proc_infos)
            }
            Err(_) => (0, Vec::new()),
        };

        Ok(GpuMetrics {
            index,
            name: info.name,
            uuid: info.uuid,
            is_mig_instance: is_mig,
            parent_gpu_index: parent_index,
            gpu_util,
            memory_util,
            sm_util: None,
            memory_used,
            memory_total,
            temperature,
            power_usage,
            power_limit,
            process_count,
            top_processes,
            architecture: info.architecture,
            compute_capability: info.compute_capability,
            ecc_enabled: info.ecc_enabled,
            temp_shutdown: info.temp_shutdown,
            temp_slowdown: info.temp_slowdown,
            clock_graphics_mhz,
            clock_sm_mhz,
            clock_memory_mhz,
            pcie_tx_kbps,
            pcie_rx_kbps,
            pcie_gen,
            pcie_width,
            performance_state,
            throttle_reasons,
            encoder_util,
            decoder_util,
            ecc_errors_corrected,
            ecc_errors_uncorrected,
        })
    }

    /// Read process name from /proc/{pid}/comm (Linux) or fallback
    fn process_name(pid: u32) -> String {
        #[cfg(target_os = "linux")]
        {
            if let Ok(name) = std::fs::read_to_string(format!("/proc/{pid}/comm")) {
                let name = name.trim();
                if !name.is_empty() {
                    return name.to_string();
                }
            }
        }
        format!("pid:{pid}")
    }

    pub fn driver_version(&self) -> String {
        self.nvml
            .sys_driver_version()
            .unwrap_or_else(|_| "Unknown".to_string())
    }

    pub fn cuda_version(&self) -> String {
        self.nvml
            .sys_cuda_driver_version()
            .map(|v| {
                let major = v / 1000;
                let minor = (v % 1000) / 10;
                format!("{major}.{minor}")
            })
            .unwrap_or_else(|_| "Unknown".to_string())
    }
}

impl Drop for NvmlCollector {
    fn drop(&mut self) {
        let prev_map = self.gpm_prev_samples.borrow();
        for &sample in prev_map.values() {
            if !sample.is_null() {
                unsafe {
                    let _ = self.raw_lib.nvmlGpmSampleFree(sample);
                }
            }
        }
    }
}

fn format_pstate(ps: PerformanceState) -> &'static str {
    match ps {
        PerformanceState::Zero => "P0",
        PerformanceState::One => "P1",
        PerformanceState::Two => "P2",
        PerformanceState::Three => "P3",
        PerformanceState::Four => "P4",
        PerformanceState::Five => "P5",
        PerformanceState::Six => "P6",
        PerformanceState::Seven => "P7",
        PerformanceState::Eight => "P8",
        PerformanceState::Nine => "P9",
        PerformanceState::Ten => "P10",
        PerformanceState::Eleven => "P11",
        PerformanceState::Twelve => "P12",
        PerformanceState::Thirteen => "P13",
        PerformanceState::Fourteen => "P14",
        PerformanceState::Fifteen => "P15",
        PerformanceState::Unknown => "P?",
    }
}

fn format_throttle_reasons(tr: ThrottleReasons) -> String {
    if tr.is_empty() || tr == ThrottleReasons::NONE {
        return String::from("None");
    }

    let mut s = String::with_capacity(48);
    macro_rules! check {
        ($flag:expr, $name:expr) => {
            if tr.contains($flag) {
                if !s.is_empty() {
                    s.push_str(", ");
                }
                s.push_str($name);
            }
        };
    }
    check!(ThrottleReasons::GPU_IDLE, "Idle");
    check!(ThrottleReasons::APPLICATIONS_CLOCKS_SETTING, "AppClk");
    check!(ThrottleReasons::SW_POWER_CAP, "SwPwrCap");
    check!(ThrottleReasons::HW_SLOWDOWN, "HW-Slow");
    check!(ThrottleReasons::SYNC_BOOST, "SyncBoost");
    check!(ThrottleReasons::SW_THERMAL_SLOWDOWN, "SW-Therm");
    check!(ThrottleReasons::HW_THERMAL_SLOWDOWN, "HW-Therm");
    check!(ThrottleReasons::HW_POWER_BRAKE_SLOWDOWN, "HW-PwrBrake");
    check!(ThrottleReasons::DISPLAY_CLOCK_SETTING, "DispClk");

    if s.is_empty() {
        String::from("Unknown")
    } else {
        s
    }
}

fn format_architecture(arch: DeviceArchitecture) -> &'static str {
    match arch {
        DeviceArchitecture::Kepler => "Kepler",
        DeviceArchitecture::Maxwell => "Maxwell",
        DeviceArchitecture::Pascal => "Pascal",
        DeviceArchitecture::Volta => "Volta",
        DeviceArchitecture::Turing => "Turing",
        DeviceArchitecture::Ampere => "Ampere",
        DeviceArchitecture::Ada => "Ada",
        DeviceArchitecture::Hopper => "Hopper",
        // Blackwell: nvml-wrapper v0.10 doesn't have this variant yet;
        // when the crate adds it, uncomment below.
        // DeviceArchitecture::Blackwell => "Blackwell",
        _ => "Unknown",
    }
}
