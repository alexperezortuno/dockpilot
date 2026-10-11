use std::{env, path::PathBuf};
use sysinfo::{Disks, Networks, System};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiskInfo {
    pub name: String,
    pub mount_point: String,
    pub total: u64,
    pub available: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NetworkInfo {
    pub name: String,
    pub addresses: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SystemSnapshot {
    pub os: String,
    pub kernel: String,
    pub hostname: String,
    pub architecture: String,
    pub uptime_seconds: u64,
    pub cpu_model: String,
    pub cpu_count: usize,
    pub cpu_usage: f32,
    pub load_one: f64,
    pub load_five: f64,
    pub load_fifteen: f64,
    pub process_count: usize,
    pub memory_total: u64,
    pub memory_used: u64,
    pub swap_total: u64,
    pub swap_used: u64,
    pub disks: Vec<DiskInfo>,
    pub networks: Vec<NetworkInfo>,
    pub dockpilot_version: String,
    pub process_id: u32,
    pub current_dir: String,
}

pub fn collect() -> SystemSnapshot {
    let mut system = System::new_all();
    system.refresh_all();

    let disks = Disks::new_with_refreshed_list()
        .iter()
        .map(|disk| DiskInfo {
            name: disk.name().to_string_lossy().into_owned(),
            mount_point: disk.mount_point().to_string_lossy().into_owned(),
            total: disk.total_space(),
            available: disk.available_space(),
        })
        .collect();
    let networks = Networks::new_with_refreshed_list()
        .iter()
        .map(|(name, network)| NetworkInfo {
            name: name.clone(),
            addresses: network
                .ip_networks()
                .iter()
                .map(|address| address.addr.to_string())
                .collect(),
        })
        .collect();
    let load = System::load_average();

    SystemSnapshot {
        os: format_os(),
        kernel: System::kernel_version().unwrap_or_else(|| "unknown".to_string()),
        hostname: System::host_name().unwrap_or_else(|| "unknown".to_string()),
        architecture: System::cpu_arch(),
        uptime_seconds: System::uptime(),
        cpu_model: system
            .cpus()
            .first()
            .map(|cpu| cpu.brand().to_string())
            .unwrap_or_else(|| "unknown".to_string()),
        cpu_count: system.cpus().len(),
        cpu_usage: system.global_cpu_usage(),
        load_one: load.one,
        load_five: load.five,
        load_fifteen: load.fifteen,
        process_count: system.processes().len(),
        memory_total: system.total_memory(),
        memory_used: system.used_memory(),
        swap_total: system.total_swap(),
        swap_used: system.used_swap(),
        disks,
        networks,
        dockpilot_version: env!("CARGO_PKG_VERSION").to_string(),
        process_id: std::process::id(),
        current_dir: env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("unknown"))
            .display()
            .to_string(),
    }
}

fn format_os() -> String {
    let name = System::name().unwrap_or_else(|| "unknown OS".to_string());
    match System::os_version() {
        Some(version) => format!("{} {}", name, version),
        None => name,
    }
}

#[cfg(test)]
mod tests {
    use super::collect;

    #[test]
    fn collects_local_identity_and_runtime_metadata() {
        let snapshot = collect();

        assert!(!snapshot.os.is_empty());
        assert!(!snapshot.architecture.is_empty());
        assert!(!snapshot.dockpilot_version.is_empty());
        assert!(snapshot.cpu_count > 0);
    }
}
