#[derive(Debug, Clone)]
pub struct SchedulerHints {
    pub time_quantum_ms: u32,
    pub priority_boost: f32,
    pub active_tasks: u32,
    pub cpu_affinity: CpuAffinity,
}

#[derive(Debug, Clone, Copy)]
pub enum CpuAffinity {
    Any,
    Performance,
    Efficiency,
}

impl Default for SchedulerHints {
    fn default() -> Self {
        SchedulerHints {
            time_quantum_ms: 20,
            priority_boost: 1.0,
            active_tasks: 0,
            cpu_affinity: CpuAffinity::Any,
        }
    }
}
