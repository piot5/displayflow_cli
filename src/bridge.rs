use std::io::{self, Write};
use serde::Serialize;

pub enum OutMode {
    Console,
    Pipe,
}

pub struct OutputManager {
    pub mode: OutMode,
}

impl OutputManager {
    pub fn new(is_daemon: bool) -> Self {
        Self {
            mode: if is_daemon { OutMode::Pipe } else { OutMode::Console },
        }
    }

    pub fn output<T: Serialize>(&self, tag: &str, data: &T) {
        match self.mode {
            OutMode::Console => {
                match tag {
                    "SCAN_RES" => {
                        if let Ok(val) = serde_json::to_value(data) {
                            let id = val["persistent_id"].as_u64().unwrap_or(0);
                            
                            // Nutze position_instance für das Kürzel
                            let hw_id = val["position_instance"].as_str().unwrap_or("");
                            let vendor = if hw_id.contains("DISPLAY") {
                                hw_id.split('\\').nth(1).and_then(|s| s.get(0..3)).unwrap_or("UNK")
                            } else {
                                "GEN"
                            };
                            
                            let res = val["resolution"].as_str().unwrap_or("N/A");
                            let freq = val["freq"].as_str().unwrap_or("N/A");
                            let active = val["active"].as_str().unwrap_or("N/A");
                            let serial = val["serial"].as_str().unwrap_or("N/A");
                            
                            let bright = val["ddc"]["brightness"].as_u64()
                                .map(|v| v.to_string()).unwrap_or_else(|| "--".into());
                            let contrast = val["ddc"]["contrast"].as_u64()
                                .map(|v| v.to_string()).unwrap_or_else(|| "--".into());

                            println!("{:<3} | {:<6} | {:<12} | {:<6} | {:<8} | {:<14} | B:{:>3} C:{:>3}", 
                                id, vendor, res, freq, active, serial, bright, contrast);
                        }
                    },
                    "SUITES_RES" => {
                        println!("\n[ KONFIGURIERTE SUITES ]");
                        println!("{:-<85}", "");
                        if let Ok(val) = serde_json::to_value(data) {
                            if let Some(list) = val.as_array() {
                                for suite_val in list {
                                    // Name aus dem Objekt oder direkt als String
                                    let name = suite_val["name"].as_str()
                                        .or(suite_val.as_str())
                                        .unwrap_or("Unknown");
                                    
                                    let config_str = suite_val["config"].as_str().unwrap_or("");
                                    
                                    let mut tasks = "-".to_string();
                                    let mut hotkey = "Keiner".to_string();

                                    // Parsing der Semikolon-getrennten Registry-Werte
                                    for part in config_str.split(';') {
                                        if let Some(t) = part.strip_prefix("tasks|") { tasks = t.to_string(); }
                                        else if let Some(h) = part.strip_prefix("hotkey|") { hotkey = h.to_string(); }
                                    }

                                    // Kürzen der Konfig-Anzeige für die Tabelle
                                    let display_config = if tasks.len() > 40 {
                                        format!("{}...", &tasks[..37])
                                    } else {
                                        tasks
                                    };

                                    println!("Suite: {:<18} | Hotkey: {:<12} | Konfig: {}", 
                                        name, hotkey, display_config);
                                }
                            }
                        }
                        println!("{:-<85}\n", "");
                    },
                    _ => {
                        let json = serde_json::to_string_pretty(data).unwrap_or_default();
                        println!("[{}] {}", tag, json);
                    }
                }
            }
            OutMode::Pipe => {
                if let Ok(json) = serde_json::to_string(data) {
                    let mut stdout = io::stdout();
                    let _ = writeln!(stdout, "{}:{}", tag, json);
                    let _ = stdout.flush();
                }
            }
        }
    }

    #[allow(dead_code)]
    pub fn raw_msg(&self, msg: &str) {
        println!("{}", msg);
        let _ = io::stdout().flush();
    }
}