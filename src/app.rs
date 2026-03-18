use std::collections::HashMap;

use crate::gpu::metrics::{GpuMetrics, MetricsHistory, SystemHistory, SystemMetrics};

const MAX_HISTORY: usize = 300; // 300 ticks = 5 min at 1000ms default interval

pub struct App {
    pub running: bool,
    pub selected_gpu: usize,
    pub metrics: Vec<GpuMetrics>,
    pub history: HashMap<String, MetricsHistory>, // keyed by UUID
    pub driver_version: String,
    pub cuda_version: String,
    pub system_metrics: Option<SystemMetrics>,
    pub system_history: SystemHistory,
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
        }
    }

    pub fn update_metrics(&mut self, mut new_metrics: Vec<GpuMetrics>) {
        // Carry forward last-known VRAM when memory_info() fails (GPM state corruption across ticks)
        for m in &mut new_metrics {
            if m.memory_used.is_none() {
                if let Some(prev) = self.metrics.iter().find(|p| p.uuid == m.uuid) {
                    m.memory_used = prev.memory_used;
                    m.memory_total = prev.memory_total;
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
        if self.history.len() > new_metrics.len() {
            self.history
                .retain(|uuid, _| new_metrics.iter().any(|m| m.uuid == *uuid));
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
