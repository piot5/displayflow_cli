use serde::{Serialize, Deserialize};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;
use chrono::Local;
use sha2::{Sha256, Digest};
use sysinfo::System; 
use serde_json::json;

use crate::scraper::{DisplayRow, DisplayTask};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TelemetryPayload {
    pub timestamp: String,
    pub device_hash: String,
    pub action: String,
    pub success: bool,
    pub detected_monitors: Vec<MonitorStats>,
    pub tasks_executed: usize,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MonitorStats {
    pub res: String,
    pub hw_id: String,
    pub is_primary: bool,
}

pub struct TelemetryManager {
    pub enabled: bool,
    storage_path: String,
}

impl TelemetryManager {
    pub fn new(enabled: bool) -> Self {
        let manager = Self {
            enabled,
            storage_path: "telemetry_improvement.json".to_string(),
        };
        if enabled { 
            manager.cleanup_old_data(); 
        }
        manager
    }

    fn generate_device_hash() -> String {
        let mut sys = System::new_all();
        sys.refresh_all();
        
        let mut hasher = Sha256::new();
        let cpu_info = sys.cpus().first().map(|c| c.brand()).unwrap_or("UnknownCPU");
        let mem_info = sys.total_memory().to_string();
        
        hasher.update(cpu_info);
        hasher.update(mem_info);
        
        format!("{:x}", hasher.finalize())
    }

    pub fn collect_system_state(
        &self, 
        action: &str, 
        success: bool, 
        rows: &[DisplayRow], 
        tasks: &[DisplayTask]
    ) {
        if !self.enabled { return; }

        let stats: Vec<MonitorStats> = rows.iter().map(|r| MonitorStats {
            res: r.resolution.clone(),
            hw_id: r.position_instance.split('\\').nth(1).unwrap_or("Unknown").to_string(),
            is_primary: r.primary == "YES",
        }).collect();

        let payload = TelemetryPayload {
            timestamp: Local::now().to_rfc3339(),
            device_hash: Self::generate_device_hash(),
            action: action.to_string(),
            success,
            detected_monitors: stats,
            tasks_executed: tasks.len(),
        };

        let _ = self.log_locally(&payload);
        self.upload_to_supabase(&payload);
    }

    fn log_locally(&self, payload: &TelemetryPayload) -> std::io::Result<()> {
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.storage_path)?;
        
        let json_line = serde_json::to_string(payload).unwrap_or_default();
        writeln!(file, "{}", json_line)
    }

    fn upload_to_supabase(&self, payload: &TelemetryPayload) {
        let client = reqwest::blocking::Client::new();
        let url = "https://pbuloekdtjpuiehbfypz.supabase.co/rest/v1/telemetry";
        
        // Liest den Key zur Laufzeit (Runtime)
        let api_key = std::env::var("SB_API_KEY").unwrap_or_else(|_| "".to_string());

        if api_key.is_empty() {
            return;
        }

        let body = json!({
            "device_hash": payload.device_hash,
            "action": payload.action,
            "success": payload.success,
            "data": payload 
        });

        let _ = client.post(url)
            .header("apikey", &api_key)
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .header("Prefer", "return=minimal") 
            .json(&body)
            .send();
    }

    fn cleanup_old_data(&self) {
        let path = Path::new(&self.storage_path);
        if let Ok(metadata) = fs::metadata(path) {
            if let Ok(modified) = metadata.modified() {
                if let Ok(elapsed) = modified.elapsed() {
                    if elapsed.as_secs() > 2592000 {
                        let _ = fs::remove_file(path);
                    }
                }
            }
        }
    }
}