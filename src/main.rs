mod engine;
mod scraper;
mod deployer;

use engine::DFEngine;
use scraper::DisplayTask;
use deployer::DeploymentManager;
use std::{env, io::{self, Write}, process};

/// STATUS_CODES:
/// 0x00: SUCCESS
/// 0x01: ERR_INVALID_INPUT
/// 0x02: ERR_DEPLOY_FAIL

fn main() {
    scraper::set_dpi_awareness();
    let args: Vec<String> = env::args().collect();
    let engine = DFEngine::new();

    if args.len() < 2 {
        execute_inventory_scan(&engine);
        return;
    }

    let mut config = RuntimeConfig::parse(&args);
    
    if config.tasks.is_empty() && config.clone_pairs.is_empty() {
        if !config.silent { eprintln!("ERR_0x01: EMPTY_TASK_SET"); }
        process::exit(1);
    }

    if config.use_hotkey {
        if !config.silent { 
            print!("REQ_HK_WAIT..."); 
            let _ = io::stdout().flush();
        }
        config.captured_hk = DeploymentManager::capture_hotkey_physical();
    }

    for (src, target) in &config.clone_pairs {
        engine.execute_clone(src, target);
    }

    if !config.tasks.is_empty() {
        engine.execute_integrated(config.tasks.clone());
    }

    if let Some(name) = config.save_name {
        if let Err(_) = DeploymentManager::create_suite(&name, &config.tasks, config.captured_hk, config.post_cmd) {
            if !config.silent { eprintln!("ERR_0x02: DEPLOY_ABORT"); }
            process::exit(1);
        }
        if !config.silent { println!("OK_SHRT_CREATED"); }
    }
}

fn execute_inventory_scan(engine: &DFEngine) {
    print!("SCAN_INIT? (y/n): ");
    let _ = io::stdout().flush();
    let mut choice = String::new();
    
    if io::stdin().read_line(&mut choice).is_ok() && choice.trim().to_lowercase() == "y" {
        let inv = engine.full_scan_discovery();
        for row in inv.iter().filter(|r| r.source == "Integrated_Final" && !r.position_instance.is_empty()) {
            let hwid = transform_hwid(&row.position_instance);
            let short_id = hwid.split('\\').nth(1).unwrap_or("UNKNOWN");
            
            if short_id == "UNKNOWN" || row.persistent_id == 0 { continue; }
            
            println!("ID:{}|RES:{}|FRQ:{}|ACT:{}|MOD:{}", 
                row.persistent_id, row.resolution, row.freq, row.active, short_id);
        }
    }
}

fn transform_hwid(raw: &str) -> String {
    if let (Some(start), Some(end)) = (raw.find('{'), raw.find('}')) {
        let mut s = raw.to_string();
        s.replace_range(start..=end, "");
        s.replace("\\\\", "\\")
    } else {
        raw.to_string()
    }
}

struct RuntimeConfig {
    tasks: Vec<DisplayTask>,
    clone_pairs: Vec<(String, String)>,
    save_name: Option<String>,
    use_hotkey: bool,
    post_cmd: Option<String>,
    silent: bool,
    captured_hk: Option<String>,
}

impl RuntimeConfig {
    fn parse(args: &[String]) -> Self {
        let mut config = Self {
            tasks: Vec::new(),
            clone_pairs: Vec::new(),
            save_name: None,
            use_hotkey: false,
            post_cmd: None,
            silent: false,
            captured_hk: None,
        };

        let mut iter = args.iter().skip(1);
        while let Some(arg) = iter.next() {
            match arg.as_str() {
                "--silent" => config.silent = true,
                "-h" | "--hotkey" => config.use_hotkey = true,
                "-c" | "--clone" => {
                    if let Some(pair) = iter.next() {
                        let parts: Vec<&str> = pair.split('/').collect();
                        if parts.len() == 2 {
                            config.clone_pairs.push((parts[0].to_string(), parts[1].to_string()));
                        }
                    }
                }
                _ if arg.starts_with("save:") => config.save_name = Some(arg[5..].to_string()),
                _ if arg.starts_with("post:") => config.post_cmd = Some(arg[5..].to_string()),
                _ => {
                    let p: Vec<&str> = arg.split(':').collect();
                    if p.len() >= 6 {
                        config.tasks.push(DisplayTask {
                            query: p[0].to_string(),
                            width: p[1].parse().unwrap_or(1920),
                            height: p[2].parse().unwrap_or(1080),
                            x: p[3].parse().unwrap_or(0),
                            y: p[4].parse().unwrap_or(0),
                            is_primary: p.get(5).map_or(false, |&v| v == "1"),
                            direction: Some(p.get(6).unwrap_or(&"0").to_string()),
                            freq: p.get(7).and_then(|s| s.parse().ok()).unwrap_or(0),
                        });
                    }
                }
            }
        }
        config
    }
}