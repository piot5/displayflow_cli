mod engine;
mod scraper;
mod deployer;

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

    if cli.tasks.is_empty() && cli.clone.is_empty() && cli.save.is_none() {
        run_scan(&engine);
        return;
    }

    let mut hk_val: Option<String> = None;
    if cli.hotkey {
        if !cli.silent { 
            print!("WAIT_HK..."); 
            let _ = io::stdout().flush();
        }
        hk_val = DeploymentManager::capture_hotkey_physical();
    }

    let mut clone_pairs = Vec::new();
    for c_str in &cli.clone {
        let bits: Vec<&str> = c_str.split('/').collect();
        if bits.len() == 2 {
            clone_pairs.push((bits[0].to_string(), bits[1].to_string()));
        }
    }

    let display_tasks: Vec<DisplayTask> = cli.tasks.iter()
        .filter_map(|s| parse_task(s))
        .collect();

    if !display_tasks.is_empty() || !clone_pairs.is_empty() {
        engine.atomic_deploy(display_tasks.clone(), clone_pairs);
    }

    if let Some(name) = &cli.save {
        if DeploymentManager::create_suite(name, &display_tasks, hk_val, cli.post).is_err() {
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

fn run_scan(eng: &DFEngine) {
    print!("SCAN? (y/n): ");
    let _ = io::stdout().flush();
    let mut buf = String::new();
    
    if io::stdin().read_line(&mut buf).is_ok() && buf.trim().eq_ignore_ascii_case("y") {
        let data = eng.full_scan_discovery();
        for r in data.iter().filter(|r| r.source == "Integrated_Final" && !r.position_instance.is_empty()) {
            let hw = clean_id(&r.position_instance);
            let sid = hw.split('\\').nth(1).unwrap_or("N/A");
            
            if sid != "N/A" && r.persistent_id != 0 {
                println!("ID:{}|RES:{}|FRQ:{}|ACT:{}|MOD:{}", 
                    r.persistent_id, r.resolution, r.freq, r.active, sid);
            }
        }
    }
}

fn clean_id(raw: &str) -> String {
    if let (Some(a), Some(b)) = (raw.find('{'), raw.find('}')) {
        let mut s = raw.to_string();
        s.replace_range(a..=b, "");
        s.replace("\\\\", "\\")
    } else {
        raw.to_string()
    }
}