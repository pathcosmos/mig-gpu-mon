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
    nvmlDeviceAttributes_t, nvmlDevice_t, nvmlProcessUtilizationSample_t, nvmlSample_t, NvmlLib,
};
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::ffi::OsStr;
use std::os::raw::c_uint;

use super::metrics::{GpuMetrics, GpuProcessInfo};

/// Write a debug line to /tmp/mig-gpu-mon-debug.log (only when --debug is active).
/// Uses append mode with per-line write to avoid buffering issues.
macro_rules! dbg_log {
    ($($arg:tt)*) => {
        if crate::DEBUG_MODE.load(std::sync::atomic::Ordering::Relaxed) {
            use std::io::Write;
            if let Ok(mut f) = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open("/tmp/mig-gpu-mon-debug.log")
            {
                let _ = writeln!(f, "[{:.3}] {}",
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs_f64() % 100000.0,
                    format_args!($($arg)*)
                );
            }
        }
    };
}

/// Cached static device info that never changes at runtime.
/// Uses Rc<str> for string fields to make clone() cheap (pointer bump, no heap alloc).
#[derive(Clone)]
struct DeviceInfo {
    name: std::rc::Rc<str>,
    uuid: std::rc::Rc<str>,
    architecture: Option<&'static str>,
    compute_capability: Option<std::rc::Rc<str>>,
    ecc_enabled: Option<bool>,
    temp_shutdown: Option<u32>,
    temp_slowdown: Option<u32>,
    /// MIG GPU instance slice count (e.g. 3 for 3g.40gb) — only set on MIG devices
    gpu_instance_slice_count: Option<u32>,
    /// MIG GPU instance ID (for process distribution by gpu_instance_id)
    gpu_instance_id: Option<u32>,
    /// Pre-formatted MIG display name (cached to avoid per-tick Rc<str> allocation)
    mig_display_name: Option<std::rc::Rc<str>>,
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
    /// Cache: pid → process name (Rc<str> for zero-cost sharing with GpuProcessInfo).
    /// Entries are pruned each tick to remove dead PIDs.
    proc_name_cache: RefCell<HashMap<u32, std::rc::Rc<str>>>,
    /// Reusable set for tracking active device handles during collection (O(1) lookup in prune)
    active_handles: RefCell<HashSet<usize>>,
    /// Reusable set for PID deduplication during process collection (avoids per-tick alloc)
    proc_seen_pids: RefCell<HashSet<u32>>,
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

        let collector = Self {
            nvml,
            raw_lib,
            device_cache: RefCell::new(HashMap::new()),
            proc_sample_buf: RefCell::new(Vec::with_capacity(32)),
            sample_buf: RefCell::new(Vec::with_capacity(128)),
            proc_name_cache: RefCell::new(HashMap::new()),
            active_handles: RefCell::new(HashSet::with_capacity(32)),
            proc_seen_pids: RefCell::new(HashSet::with_capacity(16)),
        };
        dbg_log!("=== NvmlCollector::new() ===");
        dbg_log!("driver={}, cuda={}",
            collector.nvml.sys_driver_version().unwrap_or_else(|_| "?".into()),
            collector.nvml.sys_cuda_driver_version().map(|v| format!("{}.{}", v/1000, (v%1000)/10)).unwrap_or_else(|_| "?".into()));
        if let Ok(cnt) = collector.nvml.device_count() {
            dbg_log!("device_count={cnt}");
        }
        Ok(collector)
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
        // Track active device handles to prune stale cache entries (MIG reconfig / hot-remove)
        let mut active_handles = self.active_handles.borrow_mut();
        active_handles.clear();

        for i in 0..device_count {
            let device = match self.nvml.device_by_index(i) {
                Ok(d) => d,
                Err(_) => continue, // Skip failed device — don't fail all GPUs
            };
            active_handles.insert(unsafe { device.handle() } as usize);

            if self.is_mig_enabled(&device) {
                if let Ok(mig_metrics) = self.collect_mig_instances(&device, i, &mut active_handles)
                {
                    all_metrics.extend(mig_metrics);
                }
            }

            if let Ok(metrics) = self.collect_device_metrics(&device, i, false, None) {
                all_metrics.push(metrics);
            }
        }

        // Prune stale cache entries for removed GPUs / reconfigured MIG instances.
        // Prevents unbounded HashMap growth and frees stale cached resources.
        self.prune_stale_caches(&active_handles);
        drop(active_handles);

        // Prune process name cache — keep only PIDs seen this tick.
        // Reuse proc_seen_pids (no longer borrowed after collect phases) to avoid
        // allocating a fresh HashSet every tick.
        {
            let mut name_cache = self.proc_name_cache.borrow_mut();
            if !name_cache.is_empty() {
                let mut active_pids = self.proc_seen_pids.borrow_mut();
                active_pids.clear();
                active_pids.extend(
                    all_metrics
                        .iter()
                        .flat_map(|m| m.top_processes.iter().map(|p| p.pid)),
                );
                name_cache.retain(|pid, _| active_pids.contains(pid));
                let target = name_cache.len().max(16) * 2;
                if name_cache.capacity() > target * 2 {
                    name_cache.shrink_to(target);
                }
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
        active_handles: &mut HashSet<usize>,
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

            // Pre-fetch parent-level utilization via samples (for slice-ratio fallback)
            let parent_sampled_util = self.get_gpu_util_from_samples(parent_handle);
            let parent_sampled_mem_util = self.get_mem_util_from_samples(parent_handle);
            // Fallback: parent's standard utilization_rates().memory (works on drivers
            // where nvmlDeviceGetSamples(type=1) is unsupported)
            let parent_util_rates = parent_device.utilization_rates().ok();
            let parent_util_rates_mem: Option<u32> = parent_util_rates.as_ref().map(|u| u.memory);
            // MaxMigDeviceCount = total number of GPU instance slices (e.g. 7 for H100)
            let total_slices = max_count;

            dbg_log!("--- collect_mig GPU {gpu_index} ---");
            dbg_log!("  parent_sampled_gpu_util={:?}", parent_sampled_util);
            dbg_log!("  parent_sampled_mem_util={:?}", parent_sampled_mem_util);
            dbg_log!("  parent_util_rates={}", match &parent_util_rates {
                Some(u) => format!("gpu={}% mem={}%", u.gpu, u.memory),
                None => "FAIL".to_string(),
            });
            dbg_log!("  total_slices(max_mig_count)={total_slices}");

            // === Phase 1: Collect base metrics (VRAM, utilization) for all MIG instances ===
            // (mig_handle, gpu_instance_id, metrics) — cache gi_id to avoid
            // redundant Device::new + get_device_info in Phase 1.5 and Phase 2.
            let mut phase1: Vec<(nvmlDevice_t, Option<u32>, GpuMetrics)> =
                Vec::with_capacity(max_count as usize);

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
                active_handles.insert(mig_handle as usize);

                let mig_device = Device::new(mig_handle, &self.nvml);
                let mig_info = self.get_device_info(&mig_device, true);
                let gi_id = mig_info.gpu_instance_id;

                if let Ok(mut metrics) =
                    self.collect_device_metrics(&mig_device, mig_idx, true, Some(gpu_index))
                {
                    dbg_log!("  MIG {mig_idx}: slice_count={:?} gi_id={:?} gpu_util={:?} mem_util={:?} vram_used={:?}",
                        mig_info.gpu_instance_slice_count, mig_info.gpu_instance_id,
                        metrics.gpu_util, metrics.memory_util, metrics.memory_used);

                    // Fallback 1: process utilization (no GPM involvement)
                    if metrics.gpu_util.is_none() {
                        let proc_util = self.get_process_utilization(mig_handle);
                        dbg_log!("  MIG {mig_idx}: process_util fallback={:?}", proc_util);
                        if let Some((sm, mem)) = proc_util {
                            metrics.gpu_util = Some(sm);
                            metrics.sm_util = Some(sm);
                            if mem > 0 {
                                metrics.memory_util = Some(mem);
                            }
                        }
                    }

                    // Fallback 2: scale parent's sampled util by MIG slice ratio (no GPM)
                    if metrics.gpu_util.is_none() {
                        if let Some(p_util) = parent_sampled_util {
                            if let Some(mig_slices) = mig_info.gpu_instance_slice_count {
                                let scaled = ((p_util as u64) * (total_slices as u64)
                                    / (mig_slices as u64))
                                    .min(100) as u32;
                                dbg_log!("  MIG {mig_idx}: parent-scale gpu_util: parent={p_util} → scaled={scaled}");
                                metrics.gpu_util = Some(scaled);
                                metrics.sm_util = Some(scaled);
                            }
                        }
                    }

                    // Fallback for Mem Ctrl: scale parent's memory controller util
                    if metrics.memory_util.is_none() {
                        if let Some(p_mem_util) = parent_sampled_mem_util {
                            if let Some(mig_slices) = mig_info.gpu_instance_slice_count {
                                let scaled = ((p_mem_util as u64) * (total_slices as u64)
                                    / (mig_slices as u64))
                                    .min(100) as u32;
                                dbg_log!("  MIG {mig_idx}: parent-sample mem_util: parent={p_mem_util} → scaled={scaled}");
                                metrics.memory_util = Some(scaled);
                            } else {
                                dbg_log!("  MIG {mig_idx}: parent-sample mem_util: SKIP (no slice_count)");
                            }
                        } else if let Some(p_rates_mem) = parent_util_rates_mem {
                            if let Some(mig_slices) = mig_info.gpu_instance_slice_count {
                                let scaled = ((p_rates_mem as u64) * (total_slices as u64)
                                    / (mig_slices as u64))
                                    .min(100) as u32;
                                dbg_log!("  MIG {mig_idx}: parent-rates mem_util: parent={p_rates_mem} → scaled={scaled}");
                                metrics.memory_util = Some(scaled);
                            } else {
                                dbg_log!("  MIG {mig_idx}: parent-rates mem_util: SKIP (no slice_count)");
                            }
                        } else {
                            dbg_log!("  MIG {mig_idx}: mem_util: ALL parent fallbacks FAILED (sampled=None, rates=None)");
                        }
                    }

                    // Cache MIG display name to avoid per-tick Rc<str> allocation
                    let mig_key = mig_handle as usize;
                    let cached_name = {
                        let cache = self.device_cache.borrow();
                        cache
                            .get(&mig_key)
                            .and_then(|info| info.mig_display_name.clone())
                    };
                    metrics.name = match cached_name {
                        Some(name) => name,
                        None => {
                            let formatted: std::rc::Rc<str> =
                                format!("MIG {mig_idx} (GPU {gpu_index}: {})", metrics.name).into();
                            if let Some(info) = self.device_cache.borrow_mut().get_mut(&mig_key) {
                                info.mig_display_name = Some(formatted.clone());
                            }
                            formatted
                        }
                    };
                    phase1.push((mig_handle, gi_id, metrics));
                }
            }

            // === Phase 2: Process fallback from parent device ===
            // MIG device handles often fail running_compute_processes() / running_graphics_processes()
            // on drivers like 535.x. Query the parent device instead and filter by gpu_instance_id.
            {
                let mut parent_procs: Vec<(u32, Option<u64>, Option<u32>)> = Vec::with_capacity(16);
                let mut seen_pids = self.proc_seen_pids.borrow_mut();
                seen_pids.clear();

                if let Ok(procs) = parent_device.running_compute_processes() {
                    for p in &procs {
                        let vram = match p.used_gpu_memory {
                            UsedGpuMemory::Used(bytes) => Some(bytes),
                            UsedGpuMemory::Unavailable => None,
                        };
                        if seen_pids.insert(p.pid) {
                            parent_procs.push((p.pid, vram, p.gpu_instance_id));
                        }
                    }
                }
                if let Ok(procs) = parent_device.running_graphics_processes() {
                    for p in &procs {
                        let vram = match p.used_gpu_memory {
                            UsedGpuMemory::Used(bytes) => Some(bytes),
                            UsedGpuMemory::Unavailable => None,
                        };
                        if seen_pids.insert(p.pid) {
                            parent_procs.push((p.pid, vram, p.gpu_instance_id));
                        }
                    }
                }

                // Release PID dedup set — no longer needed after parent process collection
                drop(seen_pids);

                // Check if any parent process has a gpu_instance_id set
                let any_gi_available = parent_procs.iter().any(|(_, _, gi)| gi.is_some());

                // Distribute parent processes to MIG instances by gpu_instance_id
                for (mig_handle, gi_id, metrics) in &mut phase1 {
                    // Skip if MIG handle already has processes (some drivers do work)
                    if !metrics.top_processes.is_empty() {
                        continue;
                    }
                    let _ = mig_handle; // used only for identity in earlier phases

                    let mut entries: Vec<(u32, Option<u64>)> =
                        if any_gi_available && gi_id.is_some() {
                            // Normal path: filter by matching gpu_instance_id
                            parent_procs
                                .iter()
                                .filter(|(_, _, proc_gi)| *proc_gi == *gi_id)
                                .map(|(pid, vram, _)| (*pid, *vram))
                                .collect()
                        } else {
                            // Fallback: driver doesn't provide gpu_instance_id —
                            // show all parent processes (better than showing nothing)
                            parent_procs
                                .iter()
                                .map(|(pid, vram, _)| (*pid, *vram))
                                .collect()
                        };

                    // Sort: known VRAM descending, Unavailable at end
                    entries.sort_by(|a, b| match (b.1, a.1) {
                        (Some(bv), Some(av)) => bv.cmp(&av),
                        (Some(_), None) => std::cmp::Ordering::Less,
                        (None, Some(_)) => std::cmp::Ordering::Greater,
                        (None, None) => a.0.cmp(&b.0),
                    });
                    entries.truncate(5);

                    let count = entries.len() as u32;
                    let proc_infos: Vec<GpuProcessInfo> = entries
                        .into_iter()
                        .map(|(pid, vram)| GpuProcessInfo {
                            pid,
                            name: self.process_name(pid),
                            vram_used: vram,
                        })
                        .collect();

                    metrics.process_count = count;
                    metrics.top_processes = proc_infos;
                }
            }

            for (_, _, m) in &phase1 {
                dbg_log!("  FINAL MIG {}: vram_used={:?} vram_total={:?} gpu_util={:?} mem_util={:?}",
                    m.name, m.memory_used.map(|v| v / 1048576), m.memory_total.map(|v| v / 1048576),
                    m.gpu_util, m.memory_util);
            }
            mig_metrics.extend(phase1.into_iter().map(|(_, _, m)| m));
        }

        Ok(mig_metrics)
    }

    /// Get aggregated process utilization via raw NVML API.
    /// Returns None on API failure (distinguishable from idle 0%).
    /// Reuses internal buffer to avoid allocation per call.
    unsafe fn get_process_utilization(&self, device_handle: nvmlDevice_t) -> Option<(u32, u32)> {
        let mut count: c_uint = 0;
        let ret = self.raw_lib.nvmlDeviceGetProcessUtilization(
            device_handle,
            std::ptr::null_mut(),
            &mut count,
            0,
        );

        if ret != 7 || count == 0 {
            return None;
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
            return None;
        }

        // Shrink buffer if it grew much larger than needed (prevent unbounded growth).
        // Use capacity > threshold*8 to avoid shrink/resize thrashing on oscillating process counts.
        let actual = count as usize;
        let floor = actual.max(32);
        if buf.capacity() > floor * 8 {
            buf.truncate(floor);
            buf.shrink_to(floor * 2);
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

        Some((max_sm, max_mem))
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

        // Shrink buffer if it grew much larger than needed (prevent unbounded growth).
        // Use capacity > threshold*8 to avoid shrink/resize thrashing on oscillating sample counts.
        let actual = count as usize;
        let floor = actual.max(128);
        if buf.capacity() > floor * 8 {
            buf.truncate(floor);
            buf.shrink_to(floor * 2);
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

    /// Get memory controller utilization from nvmlDeviceGetSamples (parent device only).
    /// Same approach as get_gpu_util_from_samples but queries memory utilization samples.
    /// Works on ALL architectures (Ampere, Hopper) without GPM dependency.
    /// Falls back gracefully on unsupported drivers (returns None).
    unsafe fn get_mem_util_from_samples(&self, device_handle: nvmlDevice_t) -> Option<u32> {
        let mut val_type: c_uint = 0;
        let mut count: c_uint = 0;

        // Size query — sample type 1 = memory controller utilization
        let ret = self.raw_lib.nvmlDeviceGetSamples(
            device_handle,
            1,
            0,
            &mut val_type,
            &mut count,
            std::ptr::null_mut(),
        );

        dbg_log!("    get_mem_util_from_samples: size_query ret={ret} count={count}");
        if (ret != 0 && ret != 7) || count == 0 {
            dbg_log!("    get_mem_util_from_samples: FAIL (ret={ret}, count={count})");
            return None;
        }

        let mut buf = self.sample_buf.borrow_mut();
        let needed = count as usize;
        if buf.len() < needed {
            buf.resize(needed, std::mem::zeroed());
        }

        let ret = self.raw_lib.nvmlDeviceGetSamples(
            device_handle,
            1,
            0,
            &mut val_type,
            &mut count,
            buf.as_mut_ptr(),
        );
        if ret != 0 || count == 0 {
            dbg_log!("    get_mem_util_from_samples: data_fetch FAIL ret={ret} count={count}");
            return None;
        }

        let actual = count as usize;
        let floor = actual.max(128);
        if buf.capacity() > floor * 8 {
            buf.truncate(floor);
            buf.shrink_to(floor * 2);
        }

        let n = actual.min(5);
        let start = actual - n;
        let sum: u64 = buf[start..actual]
            .iter()
            .map(|s| s.sampleValue.uiVal as u64)
            .sum();
        let avg = sum / n as u64;

        let util = (avg / 10000).min(100) as u32;
        dbg_log!("    get_mem_util_from_samples: OK avg_raw={avg} → util={util}%");
        Some(util)
    }

    /// Get cached or fetch device info. Static fields never change at runtime.
    fn get_device_info(&self, device: &Device, _is_mig: bool) -> DeviceInfo {
        let key = unsafe { device.handle() } as usize;
        let cache = self.device_cache.borrow();
        if let Some(info) = cache.get(&key) {
            return info.clone();
        }
        drop(cache);

        let info = DeviceInfo {
            name: device
                .name()
                .unwrap_or_else(|_| "Unknown GPU".to_string())
                .into(),
            uuid: device.uuid().unwrap_or_else(|_| "N/A".to_string()).into(),
            architecture: device.architecture().ok().map(format_architecture),
            compute_capability: device
                .cuda_compute_capability()
                .ok()
                .map(|cc| std::rc::Rc::from(format!("{}.{}", cc.major, cc.minor))),
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
            mig_display_name: None,
        };

        dbg_log!("  cache_miss device_info: name={} uuid={} arch={:?} slice_count={:?} gi_id={:?}",
            info.name, info.uuid, info.architecture, info.gpu_instance_slice_count,
            info.gpu_instance_id);
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
        // For MIG handles, skip extended API queries in get_device_info.
        let info = self.get_device_info(device, is_mig);

        // VRAM query first
        let mem_result = device.memory_info();
        let (memory_used, memory_total) = mem_result
            .as_ref()
            .map(|m| (Some(m.used), Some(m.total)))
            .unwrap_or((None, None));

        dbg_log!("  collect_device idx={index} is_mig={is_mig} name={}", info.name);
        match &mem_result {
            Ok(m) => dbg_log!("    memory_info: used={:.1}MB total={:.1}MB",
                m.used as f64 / 1048576.0, m.total as f64 / 1048576.0),
            Err(e) => dbg_log!("    memory_info: FAIL err={e:?}"),
        }

        let util_result = device.utilization_rates();
        let (gpu_util, memory_util): (Option<u32>, Option<u32>) = match &util_result {
            Ok(u) => {
                dbg_log!("    utilization_rates: gpu={}% mem={}%", u.gpu, u.memory);
                (Some(u.gpu), Some(u.memory))
            }
            Err(e) => {
                dbg_log!("    utilization_rates: FAIL err={e:?}");
                // Fallback: try nvmlDeviceGetSamples on non-MIG (parent) devices
                let sampled = if !is_mig {
                    unsafe { self.get_gpu_util_from_samples(device.handle()) }
                } else {
                    None
                };
                dbg_log!("    fallback sampled_gpu_util={:?}", sampled);
                (sampled, None) // memory_util: parent-based fallback attempted in caller for MIG
            }
        };

        dbg_log!("    final: gpu_util={:?} mem_util={:?}", gpu_util, memory_util);

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

        // Collect both compute and graphics processes, dedup by PID
        let (process_count, top_processes) = {
            let mut seen_pids = self.proc_seen_pids.borrow_mut();
            seen_pids.clear();
            let mut entries: Vec<(u32, Option<u64>)> = Vec::with_capacity(16);

            // Compute processes (primary — PyTorch, CUDA workloads)
            if let Ok(procs) = device.running_compute_processes() {
                for p in &procs {
                    let vram = match p.used_gpu_memory {
                        UsedGpuMemory::Used(bytes) => Some(bytes),
                        UsedGpuMemory::Unavailable => None,
                    };
                    if seen_pids.insert(p.pid) {
                        entries.push((p.pid, vram));
                    }
                }
            }

            // Graphics processes (display servers, Vulkan/OpenGL without CUDA)
            if let Ok(procs) = device.running_graphics_processes() {
                for p in &procs {
                    let vram = match p.used_gpu_memory {
                        UsedGpuMemory::Used(bytes) => Some(bytes),
                        UsedGpuMemory::Unavailable => None,
                    };
                    if seen_pids.insert(p.pid) {
                        entries.push((p.pid, vram));
                    }
                }
            }

            let count = entries.len() as u32;

            // Sort: known VRAM descending first, then Unavailable at the end
            entries.sort_by(|a, b| match (b.1, a.1) {
                (Some(bv), Some(av)) => bv.cmp(&av),
                (Some(_), None) => std::cmp::Ordering::Less,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                (None, None) => a.0.cmp(&b.0), // stable by PID
            });
            entries.truncate(5);

            let proc_infos: Vec<GpuProcessInfo> = entries
                .into_iter()
                .map(|(pid, vram)| GpuProcessInfo {
                    pid,
                    name: self.process_name(pid),
                    vram_used: vram,
                })
                .collect();
            (count, proc_infos)
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

    /// Read process name from cache or /proc/{pid}/comm (Linux).
    /// Cached to avoid repeated /proc I/O for stable processes.
    fn process_name(&self, pid: u32) -> std::rc::Rc<str> {
        let cache = self.proc_name_cache.borrow();
        if let Some(name) = cache.get(&pid) {
            return name.clone(); // Rc clone = pointer bump, no heap alloc
        }
        drop(cache);

        let name: std::rc::Rc<str> = Self::read_process_name(pid).into();
        self.proc_name_cache.borrow_mut().insert(pid, name.clone());
        name
    }

    fn read_process_name(pid: u32) -> String {
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

    /// Remove cache entries for device handles that were not seen during this tick.
    /// Prevents unbounded growth of device_cache on MIG reconfig or GPU hot-remove.
    fn prune_stale_caches(&self, active_handles: &HashSet<usize>) {
        let mut cache = self.device_cache.borrow_mut();
        cache.retain(|k, _| active_handles.contains(k));
        let target = cache.len() * 2;
        if cache.capacity() > cache.len().max(16) * 4 {
            cache.shrink_to(target);
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

fn format_throttle_reasons(tr: ThrottleReasons) -> std::borrow::Cow<'static, str> {
    if tr.is_empty() || tr == ThrottleReasons::NONE {
        return std::borrow::Cow::Borrowed("None");
    }

    // Single-flag fast paths — avoid String allocation for common solo reasons
    let single_flag_checks: &[(ThrottleReasons, &'static str)] = &[
        (ThrottleReasons::GPU_IDLE, "Idle"),
        (ThrottleReasons::SW_POWER_CAP, "SwPwrCap"),
        (ThrottleReasons::HW_SLOWDOWN, "HW-Slow"),
        (ThrottleReasons::SW_THERMAL_SLOWDOWN, "SW-Therm"),
        (ThrottleReasons::HW_THERMAL_SLOWDOWN, "HW-Therm"),
    ];
    for &(flag, name) in single_flag_checks {
        if tr == flag {
            return std::borrow::Cow::Borrowed(name);
        }
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
        std::borrow::Cow::Borrowed("Unknown")
    } else {
        std::borrow::Cow::Owned(s)
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
