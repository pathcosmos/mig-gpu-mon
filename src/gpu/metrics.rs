use std::collections::VecDeque;
use std::rc::Rc;

/// 1 GiB in bytes (as f64), used for byte-to-GiB conversions.
pub const GIB_F64: f64 = 1024.0 * 1024.0 * 1024.0;

/// Per-process GPU memory usage
#[derive(Debug, Clone)]
pub struct GpuProcessInfo {
    pub pid: u32,
    pub name: String,
    /// VRAM usage in bytes — None when NVML reports Unavailable (common on MIG)
    pub vram_used: Option<u64>,
}

impl GpuProcessInfo {
    pub fn vram_used_mb(&self) -> Option<u64> {
        self.vram_used.map(|v| v / (1024 * 1024))
    }
}

/// Single GPU or MIG instance metrics snapshot.
/// Uses Rc<str> for name/uuid/compute_capability to avoid cloning heap strings
/// when metrics are passed around (cheap reference count bump).
#[derive(Debug, Clone)]
pub struct GpuMetrics {
    pub index: u32,
    pub name: Rc<str>,
    pub uuid: Rc<str>,
    pub is_mig_instance: bool,
    pub parent_gpu_index: Option<u32>,

    // Utilization (0-100%) — None when driver/MIG doesn't support the query
    pub gpu_util: Option<u32>,
    pub memory_util: Option<u32>,
    pub sm_util: Option<u32>,

    // Memory (bytes) — None when NVML memory_info() fails (GPM state corruption on MIG)
    pub memory_used: Option<u64>,
    pub memory_total: Option<u64>,

    // Temperature & Power
    pub temperature: Option<u32>,
    pub power_usage: Option<u32>, // milliwatts
    pub power_limit: Option<u32>, // milliwatts

    // Processes
    pub process_count: u32,
    /// Top processes by VRAM usage (sorted descending, max 5)
    pub top_processes: Vec<GpuProcessInfo>,

    // --- Static info (cached, collected once) ---
    pub architecture: Option<&'static str>, // "Ampere", "Hopper" etc.
    pub compute_capability: Option<Rc<str>>, // "8.0", "9.0" etc.
    pub ecc_enabled: Option<bool>,
    pub temp_shutdown: Option<u32>, // shutdown threshold °C
    pub temp_slowdown: Option<u32>, // slowdown threshold °C

    // --- Dynamic extended metrics (per tick) ---
    pub clock_graphics_mhz: Option<u32>,
    pub clock_sm_mhz: Option<u32>,
    pub clock_memory_mhz: Option<u32>,
    pub pcie_tx_kbps: Option<u32>,
    pub pcie_rx_kbps: Option<u32>,
    pub pcie_gen: Option<u32>,
    pub pcie_width: Option<u32>,
    pub performance_state: Option<&'static str>, // "P0"~"P15"
    pub throttle_reasons: Option<String>,        // "None" or "SwPwrCap, HW-Therm"
    pub encoder_util: Option<u32>,               // 0-100%
    pub decoder_util: Option<u32>,               // 0-100%
    pub ecc_errors_corrected: Option<u64>,
    pub ecc_errors_uncorrected: Option<u64>,
}

impl GpuMetrics {
    pub fn memory_used_mb(&self) -> Option<u64> {
        self.memory_used.map(|v| v / (1024 * 1024))
    }

    pub fn memory_total_mb(&self) -> Option<u64> {
        self.memory_total.map(|v| v / (1024 * 1024))
    }

    pub fn memory_percent(&self) -> Option<f64> {
        let used = self.memory_used?;
        let total = self.memory_total?;
        if total == 0 {
            return Some(0.0);
        }
        Some((used as f64 / total as f64) * 100.0)
    }

    pub fn power_usage_w(&self) -> Option<f64> {
        self.power_usage.map(|p| p as f64 / 1000.0)
    }

    pub fn power_limit_w(&self) -> Option<f64> {
        self.power_limit.map(|p| p as f64 / 1000.0)
    }

    pub fn pcie_tx_mbps(&self) -> Option<f64> {
        self.pcie_tx_kbps.map(|k| k as f64 / 1024.0)
    }

    pub fn pcie_rx_mbps(&self) -> Option<f64> {
        self.pcie_rx_kbps.map(|k| k as f64 / 1024.0)
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
    pub clock_graphics_mhz: VecDeque<u32>,
    pub pcie_tx_kbps: VecDeque<u32>,
    pub pcie_rx_kbps: VecDeque<u32>,
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
            clock_graphics_mhz: VecDeque::with_capacity(max_entries),
            pcie_tx_kbps: VecDeque::with_capacity(max_entries),
            pcie_rx_kbps: VecDeque::with_capacity(max_entries),
            max_entries,
        }
    }

    pub fn push(&mut self, metrics: &GpuMetrics) {
        Self::push_or_repeat(&mut self.gpu_util, metrics.gpu_util, self.max_entries);
        Self::push_or_repeat(&mut self.memory_util, metrics.memory_util, self.max_entries);
        Self::push_or_repeat(
            &mut self.memory_used_mb,
            metrics.memory_used_mb(),
            self.max_entries,
        );
        Self::push_or_repeat(&mut self.sm_util, metrics.sm_util, self.max_entries);
        Self::push_or_repeat(&mut self.temperature, metrics.temperature, self.max_entries);
        Self::push_or_repeat(
            &mut self.power_usage_w,
            metrics.power_usage_w(),
            self.max_entries,
        );
        Self::push_or_repeat(
            &mut self.clock_graphics_mhz,
            metrics.clock_graphics_mhz,
            self.max_entries,
        );
        Self::push_or_repeat(
            &mut self.pcie_tx_kbps,
            metrics.pcie_tx_kbps,
            self.max_entries,
        );
        Self::push_or_repeat(
            &mut self.pcie_rx_kbps,
            metrics.pcie_rx_kbps,
            self.max_entries,
        );
    }

    /// Push value if Some, otherwise repeat last known value to keep sparkline rolling.
    /// Does nothing if the metric has never been observed (no data yet).
    fn push_or_repeat<T: Copy>(buf: &mut VecDeque<T>, val: Option<T>, max: usize) {
        let v = match val {
            Some(v) => v,
            None => match buf.back() {
                Some(&last) => last,
                None => return, // never observed — don't fabricate data
            },
        };
        Self::push_ring(buf, v, max);
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
    /// RAM available in bytes (usable without swapping)
    pub ram_available: u64,
    /// RAM free in bytes (not used at all, excludes cache/buffers)
    pub ram_free: u64,
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

    pub fn ram_total_gb(&self) -> f64 {
        self.ram_total as f64 / GIB_F64
    }


    /// Decompose RAM into (used_pure_bytes, cached_bytes, free_bytes) and percentages.
    /// Returns (used_pure, cached, free, used_pct, cached_pct, free_pct, avail_gb, total_gb).
    pub fn ram_breakdown(&self) -> RamBreakdown {
        let total = self.ram_total as f64;
        let used_pure = self.ram_total.saturating_sub(self.ram_available);
        let cached_bytes = self.ram_available.saturating_sub(self.ram_free);
        let (used_pct, cached_pct, free_pct) = if total > 0.0 {
            (
                (used_pure as f64 / total * 100.0) as f32,
                (cached_bytes as f64 / total * 100.0) as f32,
                (self.ram_free as f64 / total * 100.0) as f32,
            )
        } else {
            (0.0, 0.0, 0.0)
        };
        RamBreakdown {
            used_gb: used_pure as f64 / GIB_F64,
            cached_gb: cached_bytes as f64 / GIB_F64,
            free_gb: self.ram_free as f64 / GIB_F64,
            avail_gb: self.ram_available as f64 / GIB_F64,
            total_gb: self.ram_total_gb(),
            used_pct,
            cached_pct,
            free_pct,
        }
    }
}

/// Pre-computed RAM breakdown to avoid redundant calculations across draw functions.
#[derive(Debug, Clone, Copy)]
pub struct RamBreakdown {
    pub used_gb: f64,
    pub cached_gb: f64,
    pub free_gb: f64,
    pub avail_gb: f64,
    pub total_gb: f64,
    pub used_pct: f32,
    pub cached_pct: f32,
    pub free_pct: f32,
}

/// History for system metrics
#[derive(Debug, Clone)]
pub struct SystemHistory {
    pub cpu_total: VecDeque<f32>,
    pub ram_percent: VecDeque<f64>,
    /// Non-reclaimable used % (total - available) / total * 100
    pub ram_used_pct: VecDeque<f32>,
    /// Reclaimable cache/buffers % (available - free) / total * 100
    pub ram_cached_pct: VecDeque<f32>,
    max_entries: usize,
}

impl SystemHistory {
    pub fn new(max_entries: usize) -> Self {
        Self {
            cpu_total: VecDeque::with_capacity(max_entries),
            ram_percent: VecDeque::with_capacity(max_entries),
            ram_used_pct: VecDeque::with_capacity(max_entries),
            ram_cached_pct: VecDeque::with_capacity(max_entries),
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

        let total = metrics.ram_total as f64;
        let (used_pct, cached_pct) = if total > 0.0 {
            let used = metrics.ram_total.saturating_sub(metrics.ram_available) as f64;
            let cached = metrics.ram_available.saturating_sub(metrics.ram_free) as f64;
            (
                (used / total * 100.0) as f32,
                (cached / total * 100.0) as f32,
            )
        } else {
            (0.0, 0.0)
        };

        if self.ram_used_pct.len() >= self.max_entries {
            self.ram_used_pct.pop_front();
        }
        self.ram_used_pct.push_back(used_pct);

        if self.ram_cached_pct.len() >= self.max_entries {
            self.ram_cached_pct.pop_front();
        }
        self.ram_cached_pct.push_back(cached_pct);
    }
}
