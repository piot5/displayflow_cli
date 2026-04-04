mod engine;
mod scraper;
mod deployer;
mod daemon;
mod cli;
mod bridge;

use engine::DisplayController;
use scraper::{set_dpi_awareness, DisplayTask};
use deployer::DeploymentManager;
use cli::{Cli, parse_task};
use bridge::OutputManager;
use clap::Parser;
use anyhow::Result;
use std::io::{self, Write};

fn main() -> Result<()> {
    let args = Cli::parse();
    set_dpi_awareness();
    
    let engine = DisplayController::new(); 
    let bridge = OutputManager::new(args.daemon);

    // Scan system and list available suites
    if args.scan {
        // Warning: DDC/CI queries can cause a handshake (flicker) on certain monitors
        if !args.silent {
            println!("WARNING: The scan process may cause monitors to flicker briefly (Handshake).");
            print!("Continue with scan? [y/N]: ");
            let _ = io::stdout().flush();

            let mut input = String::new();
            io::stdin().read_line(&mut input)?;
            let input = input.trim().to_lowercase();

            if input != "y" && input != "j" {
                println!("Scan cancelled.");
                return Ok(());
            }
        }

        let (data, _) = engine.inventory();
        for row in data { 
            bridge.output("SCAN_RES", &row); 
        }
        let suites = engine.list_suites();
        bridge.output("SUITES_RES", &suites);
        return Ok(());
    }

    // Run as background service
    if args.daemon {
        return daemon::start_daemon_service(engine);
    }

    // Apply specific registry configuration
    if let Some(suite_name) = args.apply_suite {
        engine.apply_registry_suite(&suite_name, args.silent)?;
        std::thread::sleep(std::time::Duration::from_millis(200));
        return Ok(());
    }

    let mut tasks: Vec<DisplayTask> = args.tasks.iter().filter_map(|s| parse_task(s)).collect();

    // Set global animation for tasks
    if let Some(ref ani) = args.animation {
        for task in &mut tasks {
            task.animation = Some(ani.clone());
        }
    }

    // Save current configuration or tasks to a suite
    if let Some(save_name) = args.save {
        if tasks.is_empty() {
            let (inv, _) = engine.inventory();
            tasks = inv.iter().map(|r| r.to_task()).collect();
        }
        
        let mut hk = None;
        if args.hotkey { 
            hk = DeploymentManager::capture_hotkey(); 
        }

        DeploymentManager::create_suite(
            &save_name, &tasks, hk, args.post, args.linkdesktop.is_some(), args.hotkey
        )?;
    } else if !tasks.is_empty() {
        // Apply display tasks immediately using the existing 'apply' method
        // Note: passing empty vector for second argument as per previous working state
        engine.apply(tasks, vec![]);
        std::thread::sleep(std::time::Duration::from_millis(200));
    }

    Ok(())
}