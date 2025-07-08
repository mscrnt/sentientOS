use sysinfo::{System, SystemExt, CpuExt, DiskExt};
use std::time::{SystemTime, UNIX_EPOCH};

/// System metrics
#[derive(Debug, Clone)]
pub struct SystemMetrics {
    pub cpu_percent: f32,
    pub memory_percent: f32,
    pub disk_usage: f32,
    pub process_count: usize,
    pub uptime: u64,
    system: System,
}

impl SystemMetrics {
    pub fn new() -> Self {
        let mut system = System::new_all();
        system.refresh_all();
        
        Self {
            cpu_percent: 0.0,
            memory_percent: 0.0,
            disk_usage: 0.0,
            process_count: 0,
            uptime: 0,
            system,
        }
    }
    
    pub fn update(&mut self) {
        self.system.refresh_cpu();
        self.system.refresh_memory();
        self.system.refresh_disks();
        self.system.refresh_processes();
        
        // CPU usage (average across all cores)
        self.cpu_percent = self.system.global_cpu_info().cpu_usage();
        
        // Memory usage
        let total_memory = self.system.total_memory();
        let used_memory = self.system.used_memory();
        self.memory_percent = if total_memory > 0 {
            (used_memory as f32 / total_memory as f32) * 100.0
        } else {
            0.0
        };
        
        // Disk usage (root filesystem)
        self.disk_usage = self.system
            .disks()
            .iter()
            .find(|disk| disk.mount_point() == std::path::Path::new("/"))
            .map(|disk| {
                let total = disk.total_space();
                let available = disk.available_space();
                if total > 0 {
                    ((total - available) as f32 / total as f32) * 100.0
                } else {
                    0.0
                }
            })
            .unwrap_or(0.0);
        
        // Process count
        self.process_count = self.system.processes().len();
        
        // Uptime
        self.uptime = self.system.uptime();
    }
}