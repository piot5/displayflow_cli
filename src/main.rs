mod engine;
mod scraper;
mod deployer;
mod hw_init;
mod daemon;

use engine::DFEngine;
use scraper::{DisplayTask, set_dpi_awareness};
use deployer::DeploymentManager;
use clap::{Parser, ArgAction};
use std::process;
use anyhow::{Context, Result};
use winreg::enums::*;
use winreg::RegKey;

#[derive(Parser, Debug)]
#[command(author, version, about = "DisplayFlow CLI", long_about = None)]
struct Cli {
    #[arg(short, long, action = ArgAction::SetTrue)] 
    silent: bool,
    
    #[arg(short, long, action = ArgAction::SetTrue)] 
    hotkey: bool,
    
    #[arg(short, long, action = ArgAction::SetTrue)] 
    scan: bool,
    
    #[arg(short, long, action = ArgAction::SetTrue)] 
    daemon: bool,

    /// Create desktop link. Use -l for plain link, -l:h to include shell hotkey.
    #[arg(short('l'), long, num_args(0..=1), default_missing_value = "plain")]
    linkdesktop: Option<String>,

    #[arg(short, long)] 
    clone: Vec<String>,
    
    #[arg(long)] 
    save: Option<String>,
    
    #[arg(long)] 
    post: Option<String>,
    
    #[arg(long)] 
    apply_suite: Option<String>,
    
    #[arg(value_name = "TASKS")] 
    tasks: Vec<String>,
}

fn main() -> Result<()> {
    set_dpi_awareness();
    let cli = Cli::parse();
    let engine = DFEngine::new();

    if cli.daemon {
        return daemon::start_daemon_service(engine);
    }

    if cli.scan {
        execute_scan_output();
        return Ok(());
    }

    if let Some(suite_name) = cli.apply_suite {
        return apply_registry_suite(&suite_name, &engine, cli.silent);
    }

    if let Some(name) = cli.save {
        let mut tasks = Vec::new();
        for t_str in &cli.tasks {
            if let Some(t) = parse_task(t_str) {
                tasks.push(t);
            }
        }

        let captured_hk = if cli.hotkey {
            println!("[INPUT] Please press your desired hotkey combination...");
            DeploymentManager::capture_hotkey()
        } else {
            None
        };

        // Mapping der CLI Flags auf Deployer Logik
        let (create_link, link_with_hotkey) = match cli.linkdesktop.as_deref() {
            Some(":h") | Some("h") => (true, true),
            Some("plain") => (true, false),
            _ => (cli.linkdesktop.is_some(), false), // Fallback für reine Existenz
        };

        DeploymentManager::create_suite(
            &name, 
            &tasks, 
            captured_hk, 
            cli.post, 
            create_link, 
            link_with_hotkey
        )?;
        
        println!("[INFO] Suite '{}' saved successfully.", name);
        return Ok(());
    }

    Ok(())
}

pub fn apply_registry_suite(name: &str, engine: &DFEngine, silent: bool) -> Result<()> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let key = hkcu.open_subkey(r"Software\DisplayFlow\Suites")
        .context("No suites found in registry")?;
    
    let config: String = key.get_value(name).context("Suite not found")?;
    let mut tasks = Vec::new();
    let mut post_cmd = None;

    for part in config.split(';') {
        let kv: Vec<&str> = part.split('|').collect();
        if kv.len() < 2 { continue; }
        match kv[0] {
            "tasks" => tasks.extend(kv[1].split(',').filter_map(parse_task)),
            "post" => post_cmd = Some(kv[1].to_string()),
            _ => {}
        }
    }

    if !tasks.is_empty() {
        engine.atomic_deploy(tasks, vec![]);
        if !silent {
            if let Some(cmd) = post_cmd {
                execute_post_cmd(&cmd);
            }
        }
    }
    Ok(())
}

pub fn execute_post_cmd(cmd: &str) {
    let _ = process::Command::new("cmd").args(["/C", cmd]).spawn();
}

pub fn parse_task(raw: &str) -> Option<DisplayTask> {
    let p: Vec<&str> = raw.split(':').collect();
    if p.len() < 5 { return None; }
    Some(DisplayTask {
        query: p[0].into(),
        width: p[1].parse().ok()?,
        height: p[2].parse().ok()?,
        x: p[3].parse().ok()?,
        y: p[4].parse().ok()?,
        is_primary: p.get(5).map_or(false, |&v| v == "1"),
        direction: Some(p.get(6).unwrap_or(&"0").to_string()),
        freq: p.get(7).and_then(|s| s.parse().ok()).unwrap_or(0),
    })
}

fn execute_scan_output() {
    let data = hw_init::run_discovery_flow();
    println!("\n--- CURRENT TOPOLOGY ---");
    for row in data {
        println!(
            "ID: {} | Res: {} | Pos: {},{} | Primary: {}", 
            row.persistent_id, row.resolution, row.x, row.y, row.is_primary
        );
    }
}