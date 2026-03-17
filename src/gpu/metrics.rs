use std::collections::VecDeque;
use std::time::Instant;

/// Per-process GPU memory usage
#[derive(Debug, Clone)]
pub struct GpuProcessInfo {
    pub pid: u32,
    pub name: String,
    pub vram_used: u64, // bytes
}

impl GpuProcessInfo {
    pub fn vram_used_mb(&self) -> u64 {
        self.vram_used / (1024 * 1024)
    }
}

/// Single GPU or MIG instance metrics snapshot
#[derive(Debug, Clone)]
pub struct GpuMetrics {
    pub index: u32,
    pub name: String,
    pub uuid: String,
    pub is_mig_instance: bool,
    pub parent_gpu_index: Option<u32>,

    // Utilization (0-100%)
    pub gpu_util: u32,
    pub memory_util: u32,
    pub sm_util: Option<u32>,

    // Memory (bytes)
    pub memory_used: u64,
    pub memory_total: u64,

    // Temperature & Power
    pub temperature: Option<u32>,
    pub power_usage: Option<u32>, // milliwatts
    pub power_limit: Option<u32>, // milliwatts

    // Processes
    pub process_count: u32,
    /// Top processes by VRAM usage (sorted descending, max 5)
    pub top_processes: Vec<GpuProcessInfo>,

    pub timestamp: Instant,
}

impl GpuMetrics {
    pub fn memory_used_mb(&self) -> u64 {
        self.memory_used / (1024 * 1024)
    }

    pub fn memory_total_mb(&self) -> u64 {
        self.memory_total / (1024 * 1024)
    }

    pub fn memory_percent(&self) -> f64 {
        if self.memory_total == 0 {
            return 0.0;
        }
        (self.memory_used as f64 / self.memory_total as f64) * 100.0
    }

    pub fn power_usage_w(&self) -> Option<f64> {
        self.power_usage.map(|p| p as f64 / 1000.0)
    }

    pub fn power_limit_w(&self) -> Option<f64> {
        self.power_limit.map(|p| p as f64 / 1000.0)
    }
}

/// Time-series history for a single GPU/MIG instance.
/// Uses VecDeque for O(1) push/pop ring buffer instead of Vec::remove(0) O(n).
#[derive(Debug, Clone)]
pub struct MetricsHistory {
    pub gpu_util: VecDeque<u32>,
    pub memory_util: VecDeque<u32>,
    pub memory_used_mb: VecDeque<u64>,
    pub sm_util: VecDeque<u32>,
    pub temperature: VecDeque<u32>,
    pub power_usage_w: VecDeque<f64>,
    max_entries: usize,
}

impl MetricsHistory {
    pub fn new(max_entries: usize) -> Self {
        Self {
            gpu_util: VecDeque::with_capacity(max_entries),
            memory_util: VecDeque::with_capacity(max_entries),
            memory_used_mb: VecDeque::with_capacity(max_entries),
            sm_util: VecDeque::with_capacity(max_entries),
            temperature: VecDeque::with_capacity(max_entries),
            power_usage_w: VecDeque::with_capacity(max_entries),
            max_entries,
        }
    }

    pub fn push(&mut self, metrics: &GpuMetrics) {
        Self::push_ring(&mut self.gpu_util, metrics.gpu_util, self.max_entries);
        Self::push_ring(&mut self.memory_util, metrics.memory_util, self.max_entries);
        Self::push_ring(
            &mut self.memory_used_mb,
            metrics.memory_used_mb(),
            self.max_entries,
        );
        if let Some(sm) = metrics.sm_util {
            Self::push_ring(&mut self.sm_util, sm, self.max_entries);
        }
        if let Some(temp) = metrics.temperature {
            Self::push_ring(&mut self.temperature, temp, self.max_entries);
        }
        if let Some(power) = metrics.power_usage_w() {
            Self::push_ring(&mut self.power_usage_w, power, self.max_entries);
        }
    }

    fn push_ring<T>(buf: &mut VecDeque<T>, val: T, max: usize) {
        if buf.len() >= max {
            buf.pop_front();
        }
        buf.push_back(val);
    }
}

/// System-level metrics (CPU + RAM)
#[derive(Debug, Clone)]
pub struct SystemMetrics {
    /// Per-core CPU usage (0.0 - 100.0)
    pub cpu_usage: Vec<f32>,
    /// Total CPU usage (0.0 - 100.0)
    pub cpu_total: f32,
    /// RAM used in bytes
    pub ram_used: u64,
    /// RAM total in bytes
    pub ram_total: u64,
    /// Swap used in bytes
    pub swap_used: u64,
    /// Swap total in bytes
    pub swap_total: u64,
}

impl SystemMetrics {
    pub fn ram_percent(&self) -> f64 {
        if self.ram_total == 0 {
            return 0.0;
        }
        (self.ram_used as f64 / self.ram_total as f64) * 100.0
    }

    pub fn swap_percent(&self) -> f64 {
        if self.swap_total == 0 {
            return 0.0;
        }
        (self.swap_used as f64 / self.swap_total as f64) * 100.0
    }

    pub fn ram_used_gb(&self) -> f64 {
        self.ram_used as f64 / (1024.0 * 1024.0 * 1024.0)
    }

    pub fn ram_total_gb(&self) -> f64 {
        self.ram_total as f64 / (1024.0 * 1024.0 * 1024.0)
    }

    pub fn swap_used_gb(&self) -> f64 {
        self.swap_used as f64 / (1024.0 * 1024.0 * 1024.0)
    }

    pub fn swap_total_gb(&self) -> f64 {
        self.swap_total as f64 / (1024.0 * 1024.0 * 1024.0)
    }
}

/// History for system metrics
#[derive(Debug, Clone)]
pub struct SystemHistory {
    pub cpu_total: VecDeque<f32>,
    pub ram_percent: VecDeque<f64>,
    max_entries: usize,
}

impl SystemHistory {
    pub fn new(max_entries: usize) -> Self {
        Self {
            cpu_total: VecDeque::with_capacity(max_entries),
            ram_percent: VecDeque::with_capacity(max_entries),
            max_entries,
        }
    }

    pub fn push(&mut self, metrics: &SystemMetrics) {
        if self.cpu_total.len() >= self.max_entries {
            self.cpu_total.pop_front();
        }
        self.cpu_total.push_back(metrics.cpu_total);

        if self.ram_percent.len() >= self.max_entries {
            self.ram_percent.pop_front();
        }
        self.ram_percent.push_back(metrics.ram_percent());
    }
}
