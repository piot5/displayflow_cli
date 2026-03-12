mod engine;
mod scraper;
mod deployer;
mod hw_init;

use engine::DFEngine;
use scraper::{DisplayTask, set_dpi_awareness};
use deployer::DeploymentManager;
use clap::{Parser, ArgAction};
use std::{io::{self, Write}, process};

#[derive(Parser, Debug)]
#[command(author, version, about = None, long_about = None)]
struct Cli {
    #[arg(short, long, action = ArgAction::SetTrue)]
    silent: bool,

    #[arg(short, long, action = ArgAction::SetTrue)]
    hotkey: bool,

    #[arg(short, long, action = ArgAction::SetTrue)]
    scan: bool,

    #[arg(short, long)]
    clone: Vec<String>,

    #[arg(long)]
    save: Option<String>,

    #[arg(long)]
    post: Option<String>,

    #[arg(value_name = "TASKS")]
    tasks: Vec<String>,
}

fn main() {
    set_dpi_awareness();
    let cli = Cli::parse();
    let engine = DFEngine::new();

    if cli.scan {
        execute_scan_output();
        return;
    }

    if cli.tasks.is_empty() && cli.clone.is_empty() && cli.save.is_none() {
        return;
    }

    let mut hk_val: Option<String> = None;
    if cli.hotkey {
        if !cli.silent { 
            print!("WAIT_HK..."); 
            let _ = io::stdout().flush();
        }
        // FIX: Aufruf an geänderten Methodennamen in deployer.rs angepasst
        hk_val = DeploymentManager::capture_hotkey_physical(); 
    }

    let display_tasks: Vec<DisplayTask> = cli.tasks.iter()
        .filter_map(|s| parse_task(s))
        .collect();

    let clone_pairs: Vec<(String, String)> = cli.clone.iter()
        .filter_map(|s| {
            let p: Vec<&str> = s.split(':').collect();
            if p.len() == 2 { Some((p[0].to_string(), p[1].to_string())) } else { None }
        })
        .collect();

    if cli.save.is_none() {
        if !display_tasks.is_empty() || !clone_pairs.is_empty() {
            engine.atomic_deploy(display_tasks, clone_pairs);
        }
    } else if let Some(name) = cli.save {
        if DeploymentManager::create_suite(&name, &display_tasks, hk_val, cli.post).is_err() {
            if !cli.silent { eprintln!("ERR_0x02"); }
            process::exit(1);
        }
        if !cli.silent { println!("OK_DEPLOY"); }
    }
}

fn parse_task(raw: &str) -> Option<DisplayTask> {
    let p: Vec<&str> = raw.split(':').collect();
    if p.len() < 5 { return None; }
    
    Some(DisplayTask {
        query: p[0].into(),
        width: p[1].parse().unwrap_or(1920),
        height: p[2].parse().unwrap_or(1080),
        x: p[3].parse().unwrap_or(0),
        y: p[4].parse().unwrap_or(0),
        is_primary: p.get(5).map_or(false, |&v| v == "1"),
        direction: Some(p.get(6).unwrap_or(&"0").to_string()),
        freq: p.get(7).and_then(|s| s.parse().ok()).unwrap_or(0),
    })
}

fn execute_scan_output() {
    let data = hw_init::run_discovery_flow();
    for r in data.iter().filter(|r| r.source == "Integrated_Final" && !r.position_instance.is_empty()) {
        let hw = clean_id(&r.position_instance);
        let sid = hw.split('\\').nth(1).unwrap_or("UNKNOWN");
        println!("{}:{}:{}:{}:{}:{}", sid, r.persistent_id, r.resolution, r.freq, r.primary, r.active);
    }
}

fn clean_id(id: &str) -> String {
    id.replace("DISPLAY\\", "").split('{').next().unwrap_or(id).trim_end_matches('\\').to_string()
}