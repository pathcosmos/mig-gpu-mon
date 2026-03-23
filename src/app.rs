use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use crate::gpu::metrics::{GpuMetrics, MetricsHistory, SystemHistory, SystemMetrics};

const MAX_HISTORY: usize = 300; // 300 ticks = 5 min at 1000ms default interval

/// Max consecutive ticks to carry forward VRAM *used* when memory_info() fails.
/// GPM (nvmlGpmMigSampleGet) can corrupt NVML state, causing memory_info() to fail
/// for extended periods (>3s). 10 ticks at 1s interval = 10s tolerance.
/// memory_total is essentially static per GPU and is carried forward indefinitely.
const VRAM_CARRY_FORWARD_TTL: u32 = 10;

pub struct App {
    pub running: bool,
    pub selected_gpu: usize,
    pub metrics: Vec<GpuMetrics>,
    pub history: HashMap<Rc<str>, MetricsHistory>, // keyed by UUID
    pub driver_version: String,
    pub cuda_version: String,
    pub system_metrics: Option<SystemMetrics>,
    pub system_history: SystemHistory,
    /// Per-GPU consecutive memory_info() failure count (keyed by UUID)
    vram_fail_count: HashMap<Rc<str>, u32>,
    /// Last known memory_total per GPU (keyed by UUID) — essentially static, carried forward
    /// indefinitely so vram_max never collapses to 1 when memory_info() temporarily fails.
    last_known_vram_total: HashMap<Rc<str>, u64>,
    /// Reusable buffer for /proc/{pid} path construction (avoids per-tick String allocation)
    proc_path_buf: String,
}

impl App {
    pub fn new(driver_version: String, cuda_version: String) -> Self {
        Self {
            running: true,
            selected_gpu: 0,
            metrics: Vec::new(),
            history: HashMap::new(),
            driver_version,
            cuda_version,
            system_metrics: None,
            system_history: SystemHistory::new(MAX_HISTORY),
            vram_fail_count: HashMap::new(),
            last_known_vram_total: HashMap::new(),
            proc_path_buf: String::with_capacity(32), // "/proc/4194304" fits easily
        }
    }

    pub fn update_metrics(&mut self, mut new_metrics: Vec<GpuMetrics>) {
        // Build O(1) lookup for previous metrics — eliminates O(n²) iter().find() per GPU
        let prev_by_uuid: HashMap<&Rc<str>, usize> = self
            .metrics
            .iter()
            .enumerate()
            .map(|(i, m)| (&m.uuid, i))
            .collect();

        for m in &mut new_metrics {
            // Cache memory_total when available (essentially static per GPU).
            // Always restore it from cache so vram_max never collapses to 1.
            if let Some(total) = m.memory_total {
                self.last_known_vram_total.insert(m.uuid.clone(), total);
            } else if let Some(&cached_total) = self.last_known_vram_total.get(&m.uuid) {
                m.memory_total = Some(cached_total);
            }

            // Carry forward memory_used with TTL — stop after VRAM_CARRY_FORWARD_TTL consecutive failures
            if m.memory_used.is_none() {
                // Avoid Rc clone on cache hit: check first, insert only on miss
                let count = if let Some(c) = self.vram_fail_count.get_mut(&m.uuid) {
                    *c += 1;
                    *c
                } else {
                    self.vram_fail_count.insert(m.uuid.clone(), 1);
                    1
                };
                if count <= VRAM_CARRY_FORWARD_TTL {
                    if let Some(&idx) = prev_by_uuid.get(&m.uuid) {
                        m.memory_used = self.metrics[idx].memory_used;
                    }
                }
                // else: TTL exceeded → leave memory_used as None → UI shows "N/A"
                // but memory_total is preserved from cache above
            } else {
                // memory_info() succeeded — reset failure counter
                self.vram_fail_count.remove(&m.uuid);
            }

            // Carry forward processes only if the PIDs are still alive.
            // Reuse proc_path_buf to avoid per-PID String allocation.
            if m.top_processes.is_empty() {
                if let Some(&idx) = prev_by_uuid.get(&m.uuid) {
                    let prev = &self.metrics[idx];
                    if !prev.top_processes.is_empty() {
                        let buf = &mut self.proc_path_buf;
                        let alive: Vec<_> = prev
                            .top_processes
                            .iter()
                            .filter(|p| {
                                use std::fmt::Write;
                                buf.clear();
                                let _ = write!(buf, "/proc/{}", p.pid);
                                std::path::Path::new(buf.as_str()).exists()
                            })
                            .cloned()
                            .collect();
                        if !alive.is_empty() {
                            m.process_count = alive.len() as u32;
                            m.top_processes = alive;
                        }
                    }
                }
            }
        }

        for m in &new_metrics {
            // entry API: single hash lookup instead of contains_key + insert + get_mut
            self.history
                .entry(m.uuid.clone())
                .or_insert_with(|| MetricsHistory::new(MAX_HISTORY))
                .push(m);
        }

        // Remove history entries for GPUs that are no longer present
        // (prevents unbounded HashMap growth on MIG reconfigs / GPU hot-remove)
        // Fast path: after entry API, history.len() >= new_metrics.len().
        // Only build HashSet when history has stale entries (GPU removed / MIG reconfig).
        if self.history.len() > new_metrics.len() {
            let uuid_set: HashSet<&Rc<str>> = new_metrics.iter().map(|m| &m.uuid).collect();
            self.history.retain(|uuid, _| uuid_set.contains(uuid));
            self.vram_fail_count
                .retain(|uuid, _| uuid_set.contains(uuid));
            self.last_known_vram_total
                .retain(|uuid, _| uuid_set.contains(uuid));
            // Shrink capacity after bulk removal (same pattern as nvml.rs caches)
            let target = self.history.len().max(8) * 2;
            if self.history.capacity() > target * 2 {
                self.history.shrink_to(target);
            }
        }

        self.metrics = new_metrics;

        // Clamp selection
        if !self.metrics.is_empty() && self.selected_gpu >= self.metrics.len() {
            self.selected_gpu = self.metrics.len() - 1;
        }
    }

    pub fn update_system_metrics(&mut self, sys: SystemMetrics) {
        self.system_history.push(&sys);
        self.system_metrics = Some(sys);
    }

    pub fn next_gpu(&mut self) {
        if !self.metrics.is_empty() {
            self.selected_gpu = (self.selected_gpu + 1) % self.metrics.len();
        }
    }

    pub fn prev_gpu(&mut self) {
        if !self.metrics.is_empty() {
            self.selected_gpu = if self.selected_gpu == 0 {
                self.metrics.len() - 1
            } else {
                self.selected_gpu - 1
            };
        }
    }

    pub fn selected_metrics(&self) -> Option<&GpuMetrics> {
        self.metrics.get(self.selected_gpu)
    }

    pub fn selected_history(&self) -> Option<&MetricsHistory> {
        self.selected_metrics()
            .and_then(|m| self.history.get(&m.uuid))
    }

    pub fn quit(&mut self) {
        self.running = false;
    }
}
