use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use crate::gpu::metrics::{GpuMetrics, MetricsHistory, SystemHistory, SystemMetrics};

const MAX_HISTORY: usize = 300; // 300 ticks = 5 min at 1000ms default interval

/// Max consecutive ticks to carry forward VRAM when memory_info() fails.
/// At default 1s interval, 3 ticks = 3 seconds of tolerance for transient failures.
const VRAM_CARRY_FORWARD_TTL: u32 = 3;

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
            proc_path_buf: String::with_capacity(32), // "/proc/4194304" fits easily
        }
    }

    pub fn update_metrics(&mut self, mut new_metrics: Vec<GpuMetrics>) {
        for m in &mut new_metrics {
            // Carry forward VRAM with TTL — stop after VRAM_CARRY_FORWARD_TTL consecutive failures
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
                    if let Some(prev) = self.metrics.iter().find(|p| p.uuid == m.uuid) {
                        m.memory_used = prev.memory_used;
                        m.memory_total = prev.memory_total;
                    }
                }
                // else: TTL exceeded → leave as None → UI shows "N/A"
            } else {
                // memory_info() succeeded — reset failure counter
                self.vram_fail_count.remove(&m.uuid);
            }

            // Carry forward processes only if the PIDs are still alive.
            // Reuse proc_path_buf to avoid per-PID String allocation.
            if m.top_processes.is_empty() {
                if let Some(prev) = self.metrics.iter().find(|p| p.uuid == m.uuid) {
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
            // Avoid uuid.clone() on every tick — only clone on first encounter
            if !self.history.contains_key(&m.uuid) {
                self.history
                    .insert(m.uuid.clone(), MetricsHistory::new(MAX_HISTORY));
            }
            self.history.get_mut(&m.uuid).unwrap().push(m);
        }

        // Remove history entries for GPUs that are no longer present
        // (prevents unbounded HashMap growth on MIG reconfigs / GPU hot-remove)
        // Pre-compute UUID set for O(1) lookups instead of O(n·m) nested iteration
        if self.history.len() != new_metrics.len()
            || {
                let uuid_set: HashSet<&Rc<str>> =
                    new_metrics.iter().map(|m| &m.uuid).collect();
                self.history.keys().any(|uuid| !uuid_set.contains(uuid))
            }
        {
            let uuid_set: HashSet<&Rc<str>> =
                new_metrics.iter().map(|m| &m.uuid).collect();
            self.history.retain(|uuid, _| uuid_set.contains(uuid));
            self.vram_fail_count
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
