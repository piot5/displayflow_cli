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

    // Falls keine Argumente übergeben wurden: Interaktiver Scan-Modus
    if args.len() < 2 {
        print_header();
        
        print!("\n[SCAN] Scan display inventory now? (y/n): ");
        let _ = io::stdout().flush();

        let mut choice = String::new();
        if io::stdin().read_line(&mut choice).is_ok() {
            if choice.trim().to_lowercase() == "y" {
                println!("[WORKING] Capturing EDID and GDI topology...");
                let inv = engine.full_scan_discovery();
                
                println!("\n{:<5} | {:<12} | {:<8} | {:<20}", "ID", "RESOLUTION", "STATUS", "HARDWARE MODEL");
                println!("{:-<75}", "");

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

                    println!("{:<5} | {:<12} | {:<8} | {:<20}", 
                        row.persistent_id, row.resolution, status, short_id);
                }
                
                println!("\n[OK] Scan complete.");
            }
        }
        return;
    }

    
    let mut tasks = Vec::new();
    let mut save_name = None;
    let mut use_hotkey = false;
    let mut post_cmd = None;
    let mut is_silent = false;
    let mut captured_hk: Option<String> = None;

    for arg in args.iter().skip(1) {
        match arg.as_str() {
            "--silent" => is_silent = true,
            "-h" | "--hotkey" => use_hotkey = true,
            _ if arg.starts_with("save:") => {
                save_name = Some(arg[5..].to_string());
            }
            _ if arg.starts_with("post:") => {
                post_cmd = Some(arg[5..].to_string());
            }
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
                    };
                    tasks.push(task);
                }
            }
        }
    }

    if tasks.is_empty() {
        if !is_silent { eprintln!("[ERROR] No valid display data provided."); }
        process::exit(1);
    }

    
    if use_hotkey {
        if !is_silent { 
            println!("\n[WAIT] Ready. Push Hotkey..."); 
            let _ = io::stdout().flush();
        }
        captured_hk = DeploymentManager::capture_hotkey_physical();
        
        if !is_silent { println!("[SIGNAL] Hotkey set."); }
    }

    
    if !is_silent { 
        println!("[EXEC] Success..."); 
    }
    
    engine.execute_integrated(tasks.clone());

    
    if let Some(name) = save_name {
        match DeploymentManager::create_suite(&name, &tasks, captured_hk, post_cmd) {
            Ok(_) => { if !is_silent { println!("[OK] Shortcut ready."); } },
            Err(e) => { eprintln!("[ERROR] Deployment aborted: {}", e); }
        }
    }
    
    if !is_silent { println!("[DONE] All Done."); }
}

fn print_header() {
    println!("====================================================");
    println!("                   DISPLAYFLOW v0.98                 ");
    println!("====================================================");
}