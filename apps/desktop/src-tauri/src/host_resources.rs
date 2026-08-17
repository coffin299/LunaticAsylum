//! ホスト PC のリソース使用率（Overview 用）

use serde::Serialize;
use sysinfo::{Disks, System};

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct HostResourcesDto {
    pub cpu_percent: f32,
    pub memory_used: u64,
    pub memory_total: u64,
    pub disk_used: u64,
    pub disk_total: u64,
}

pub fn snapshot() -> HostResourcesDto {
    let mut sys = System::new();
    sys.refresh_cpu_usage();
    std::thread::sleep(std::time::Duration::from_millis(120));
    sys.refresh_cpu_usage();
    sys.refresh_memory();

    let cpu_percent = sys.global_cpu_usage();
    let memory_used = sys.used_memory();
    let memory_total = sys.total_memory();

    let disks = Disks::new_with_refreshed_list();
    let (disk_used, disk_total) = disks.iter().fold((0u64, 0u64), |(u, t), d| {
        (
            u.saturating_add(d.total_space().saturating_sub(d.available_space())),
            t.saturating_add(d.total_space()),
        )
    });

    HostResourcesDto {
        cpu_percent,
        memory_used,
        memory_total,
        disk_used,
        disk_total,
    }
}
