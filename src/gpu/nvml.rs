use anyhow::{Context, Result};
use nvml_wrapper::enum_wrappers::device::TemperatureSensor;
use nvml_wrapper::Device;
use nvml_wrapper::Nvml;
use nvml_wrapper_sys::bindings::{nvmlDevice_t, nvmlProcessUtilizationSample_t, NvmlLib};
use std::cell::RefCell;
use std::collections::HashMap;
use std::ffi::OsStr;
use std::os::raw::c_uint;
use std::time::Instant;

use super::metrics::GpuMetrics;

/// Cached static device info that never changes at runtime
struct DeviceInfo {
    name: String,
    uuid: String,
}

pub struct NvmlCollector {
    nvml: Nvml,
    raw_lib: NvmlLib,
    /// Cache: device handle pointer → static info (name, uuid).
    /// RefCell for interior mutability — cache is populated lazily while
    /// NVML device handles borrow &self.nvml.
    device_cache: RefCell<HashMap<usize, DeviceInfo>>,
    /// Reusable buffer for process utilization samples (avoid alloc per tick)
    proc_sample_buf: RefCell<Vec<nvmlProcessUtilizationSample_t>>,
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
            let lib = Self::load_raw_lib(nvml_lib_path)
                .context("Failed to load NVML shared library")?;
            #[cfg(target_os = "windows")]
            let lib = NvmlLib::new("nvml.dll").context("Failed to load NVML DLL")?;
            lib
        };

        Ok(Self {
            nvml,
            raw_lib,
            device_cache: RefCell::new(HashMap::new()),
            proc_sample_buf: RefCell::new(Vec::with_capacity(32)),
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
        NvmlLib::new("libnvidia-ml.so.1")
            .map_err(|e| anyhow::anyhow!("{e}"))
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
            let ret = self
                .raw_lib
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
                    if metrics.gpu_util == 0 && metrics.memory_util == 0 {
                        let (sm, mem) = self.get_process_utilization(mig_handle);
                        metrics.gpu_util = sm;
                        metrics.memory_util = mem;
                        metrics.sm_util = Some(sm);
                    }

                    metrics.name =
                        format!("MIG {mig_idx} (GPU {gpu_index}: {})", metrics.name);
                    mig_metrics.push(metrics);
                }
            }
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

        let mut max_sm: u32 = 0;
        let mut max_mem: u32 = 0;
        for sample in &buf[..count as usize] {
            if sample.smUtil <= 100 {
                max_sm = max_sm.max(sample.smUtil);
            }
            if sample.memUtil <= 100 {
                max_mem = max_mem.max(sample.memUtil);
            }
        }

        (max_sm, max_mem)
    }

    /// Get cached or fetch device name/uuid. These never change at runtime.
    fn get_device_info(&self, device: &Device) -> (String, String) {
        let key = unsafe { device.handle() } as usize;
        let cache = self.device_cache.borrow();
        if let Some(info) = cache.get(&key) {
            return (info.name.clone(), info.uuid.clone());
        }
        drop(cache);

        let name = device.name().unwrap_or_else(|_| "Unknown GPU".to_string());
        let uuid = device.uuid().unwrap_or_else(|_| "N/A".to_string());
        self.device_cache.borrow_mut().insert(
            key,
            DeviceInfo {
                name: name.clone(),
                uuid: uuid.clone(),
            },
        );
        (name, uuid)
    }

    fn collect_device_metrics(
        &self,
        device: &Device,
        index: u32,
        is_mig: bool,
        parent_index: Option<u32>,
    ) -> Result<GpuMetrics> {
        let (name, uuid) = self.get_device_info(device);

        let (gpu_util, memory_util) = device
            .utilization_rates()
            .map(|u| (u.gpu, u.memory))
            .unwrap_or((0, 0));

        let (memory_used, memory_total) = device
            .memory_info()
            .map(|m| (m.used, m.total))
            .unwrap_or((0, 0));

        let temperature = device.temperature(TemperatureSensor::Gpu).ok();
        let power_usage = device.power_usage().ok();
        let power_limit = device.power_management_limit().ok();

        let process_count = device
            .running_compute_processes()
            .map(|p| p.len() as u32)
            .unwrap_or(0);

        Ok(GpuMetrics {
            index,
            name,
            uuid,
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
            timestamp: Instant::now(),
        })
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
