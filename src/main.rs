mod engine;
mod scraper;
mod deployer;

use engine::DFEngine;
use scraper::DisplayTask;
use deployer::DeploymentManager;
use std::{env, io::{self, Write}, process};

fn main() {
    scraper::set_dpi_awareness();
    let args: Vec<String> = env::args().collect();
    let engine = DFEngine::new();

    if args.len() < 2 {
        print_header();
        print!("\n[SCAN] Scan display inventory now? (y/n): ");
        let _ = io::stdout().flush();
        let mut choice = String::new();
        if io::stdin().read_line(&mut choice).is_ok() {
            if choice.trim().to_lowercase() == "y" {
                let inv = engine.full_scan_discovery();
                println!("\n{:<5} | {:<12} | {:<6} | {:<8} | {:<20}", "ID", "RESOLUTION", "FREQ", "STATUS", "HARDWARE MODEL");
                println!("{:-<85}", "");
                for row in inv.iter().filter(|r| r.source == "Integrated_Final" && !r.position_instance.is_empty()) {
                    let status = if row.active == "YES" { "ACTIVE" } else { "INACTIVE" };
                    let cleaned_hwid = if let (Some(start), Some(end)) = (row.position_instance.find('{'), row.position_instance.find('}')) {
                        let mut s = row.position_instance.clone();
                        s.replace_range(start..=end, "");
                        s.replace("\\\\", "\\")
                    } else {
                        row.position_instance.clone()
                    };
                    let short_id = cleaned_hwid.split('\\').nth(1).unwrap_or("UNKNOWN");
                    if short_id == "UNKNOWN" || row.persistent_id == 0 { continue; }
                    println!("{:<5} | {:<12} | {:<6} | {:<8} | {:<20}", 
                        row.persistent_id, row.resolution, row.freq, status, short_id);
                }
            }
        }
        return;
    }

    let mut tasks = Vec::new();
    let mut clone_pairs = Vec::new();
    let mut save_name = None;
    let mut use_hotkey = false;
    let mut post_cmd = None;
    let mut is_silent = false;
    let mut captured_hk: Option<String> = None;

    let mut iter = args.iter().skip(1);
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--silent" => is_silent = true,
            "-h" | "--hotkey" => use_hotkey = true,
            "-c" | "--clone" => {
                if let Some(pair) = iter.next() {
                    let parts: Vec<&str> = pair.split('/').collect();
                    if parts.len() == 2 {
                        clone_pairs.push((parts[0].to_string(), parts[1].to_string()));
                    }
                }
            }
            _ if arg.starts_with("save:") => { save_name = Some(arg[5..].to_string()); }
            _ if arg.starts_with("post:") => { post_cmd = Some(arg[5..].to_string()); }
            _ => {
                let p: Vec<&str> = arg.split(':').collect();
                if p.len() >= 6 {
                    let task = DisplayTask {
                        query: p[0].to_string(), 
                        width: p[1].parse().unwrap_or(1920),
                        height: p[2].parse().unwrap_or(1080),
                        x: p[3].parse().unwrap_or(0),
                        y: p[4].parse().unwrap_or(0),
                        is_primary: p[5] == "1",
                        direction: Some(p.get(6).unwrap_or(&"0").to_string()),
                        freq: p.get(7).and_then(|s| s.parse().ok()).unwrap_or(0),
                    };
                    tasks.push(task);
                }
            }
        }
    }

    if tasks.is_empty() && clone_pairs.is_empty() {
        if !is_silent { eprintln!("[ERROR] No valid display or clone data provided."); }
        process::exit(1);
    }

    if use_hotkey {
        if !is_silent { 
            println!("\n[WAIT] Ready. Push Hotkey..."); 
            let _ = io::stdout().flush();
        }
        captured_hk = DeploymentManager::capture_hotkey_physical();
    }

    for (src, target) in &clone_pairs {
        engine.execute_clone(src, target);
    }

    if !tasks.is_empty() {
        engine.execute_integrated(tasks.clone());
    }

    if let Some(name) = save_name {
        match DeploymentManager::create_suite(&name, &tasks, captured_hk, post_cmd) {
            Ok(_) => { if !is_silent { println!("[OK] Shortcut ready."); } },
            Err(e) => { if !is_silent { eprintln!("[ERROR] Deployment aborted: {}", e); } }
        }
    }
}

fn print_header() {
    println!("====================================================");
    println!("                   DISPLAYFLOW v0.98                 ");
    println!("====================================================");
}