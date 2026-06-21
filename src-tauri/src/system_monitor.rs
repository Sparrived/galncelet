use serde::Serialize;
use sysinfo::{Components, System};
use std::sync::Mutex;

/// 系统监控状态
pub struct SystemMonitorState {
    sys: Mutex<System>,
    networks: Mutex<sysinfo::Networks>,
    disks: Mutex<sysinfo::Disks>,
    last_network: Mutex<Option<NetworkSnapshot>>,
}

#[derive(Clone, Debug)]
struct NetworkSnapshot {
    upload_bytes: u64,
    download_bytes: u64,
    timestamp: std::time::Instant,
}

#[derive(Serialize, Clone, Debug)]
pub struct CpuInfo {
    pub usage: f32,
    pub cores: usize,
    pub frequency: u64,
    pub temperature: Option<f32>,
}

#[derive(Serialize, Clone, Debug)]
pub struct MemoryInfo {
    pub used: u64,
    pub total: u64,
    pub usage: f32,
}

#[derive(Serialize, Clone, Debug)]
pub struct GpuInfo {
    pub name: String,
    pub usage: f32,
    pub memory_used: u64,
    pub memory_total: u64,
    pub temperature: Option<f32>,
}

#[derive(Serialize, Clone, Debug)]
pub struct DiskInfo {
    pub used: u64,
    pub total: u64,
    pub usage: f32,
}

#[derive(Serialize, Clone, Debug)]
pub struct NetworkInfo {
    pub upload_bytes: u64,
    pub download_bytes: u64,
    pub upload_speed: u64,
    pub download_speed: u64,
}

#[derive(Serialize, Clone, Debug)]
pub struct SystemMetrics {
    pub cpu: CpuInfo,
    pub memory: MemoryInfo,
    pub gpu: Option<GpuInfo>,
    pub disk: DiskInfo,
    pub network: NetworkInfo,
}

impl SystemMonitorState {
    pub fn new() -> Self {
        let mut sys = System::new_all();
        sys.refresh_all();
        let networks = sysinfo::Networks::new_with_refreshed_list();
        let disks = sysinfo::Disks::new_with_refreshed_list();
        Self {
            sys: Mutex::new(sys),
            networks: Mutex::new(networks),
            disks: Mutex::new(disks),
            last_network: Mutex::new(None),
        }
    }

    pub fn refresh(&self) {
        let mut sys = self.sys.lock().unwrap();
        sys.refresh_all();
        let mut networks = self.networks.lock().unwrap();
        networks.refresh();
        let mut disks = self.disks.lock().unwrap();
        disks.refresh();
    }

    pub fn get_metrics(&self) -> SystemMetrics {
        let mut sys = self.sys.lock().unwrap();
        sys.refresh_cpu();
        sys.refresh_memory();

        let mut networks = self.networks.lock().unwrap();
        networks.refresh();

        let mut disks = self.disks.lock().unwrap();
        disks.refresh();

        // CPU
        let cpu_usage = sys.global_cpu_info().cpu_usage();
        let cpu_cores = sys.cpus().len();
        let cpu_frequency = sys.cpus().first().map(|c| c.frequency()).unwrap_or(0);
        let cpu_temperature = Components::new_with_refreshed_list()
            .iter()
            .find(|c| c.label().to_lowercase().contains("cpu") || c.label().to_lowercase().contains("core"))
            .map(|c| c.temperature());

        let cpu = CpuInfo {
            usage: cpu_usage,
            cores: cpu_cores,
            frequency: cpu_frequency,
            temperature: cpu_temperature,
        };

        // Memory
        let memory_used = sys.used_memory();
        let memory_total = sys.total_memory();
        let memory_usage = if memory_total > 0 {
            memory_used as f32 / memory_total as f32
        } else {
            0.0
        };

        let memory = MemoryInfo {
            used: memory_used,
            total: memory_total,
            usage: memory_usage,
        };

        // GPU - 尝试检测，但可能失败
        let gpu = None; // GPU 检测需要额外的库支持

        // Disk - 使用主磁盘
        let disk = disks.iter()
            .find(|d| d.mount_point().to_str().map(|s| s.starts_with("C:")).unwrap_or(false))
            .or_else(|| disks.iter().next())
            .map(|d| {
                let total = d.total_space();
                let available = d.available_space();
                let used = total - available;
                let usage = if total > 0 {
                    used as f32 / total as f32
                } else {
                    0.0
                };
                DiskInfo {
                    used,
                    total,
                    usage,
                }
            })
            .unwrap_or(DiskInfo {
                used: 0,
                total: 0,
                usage: 0.0,
            });

        // Network
        let (upload_bytes, download_bytes) = networks.iter()
            .fold((0u64, 0u64), |(up, down), (_, data)| {
                (up + data.total_transmitted(), down + data.total_received())
            });

        let now = std::time::Instant::now();
        let mut last_network = self.last_network.lock().unwrap();

        let (upload_speed, download_speed) = if let Some(ref last) = *last_network {
            let elapsed = now.duration_since(last.timestamp).as_secs_f64();
            if elapsed > 0.0 {
                let up_speed = ((upload_bytes.saturating_sub(last.upload_bytes)) as f64 / elapsed) as u64;
                let down_speed = ((download_bytes.saturating_sub(last.download_bytes)) as f64 / elapsed) as u64;
                (up_speed, down_speed)
            } else {
                (0, 0)
            }
        } else {
            (0, 0)
        };

        *last_network = Some(NetworkSnapshot {
            upload_bytes,
            download_bytes,
            timestamp: now,
        });

        let network = NetworkInfo {
            upload_bytes,
            download_bytes,
            upload_speed,
            download_speed,
        };

        SystemMetrics {
            cpu,
            memory,
            gpu,
            disk,
            network,
        }
    }
}

/// Tauri 命令：获取系统性能指标
#[tauri::command]
pub fn fetch_system_metrics(state: tauri::State<'_, std::sync::Arc<SystemMonitorState>>) -> SystemMetrics {
    state.get_metrics()
}
