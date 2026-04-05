mod display_controller;
mod scraper;
mod profiles;
mod daemon;
mod cli;
mod output;

use display_controller::DisplayLogic;
use scraper::{set_dpi_awareness, DisplayTask};
use profiles::ProfileManager;
use cli::{Cli, parse_task};
use output::DisplayOutput;
use clap::Parser;
use anyhow::Result;

fn main() -> Result<()> {
    let args = Cli::parse();
    set_dpi_awareness();
    
    let display_controller = DisplayLogic::new(); 
    let output = DisplayOutput::new(args.daemon);

    // Scan system and list available suites
    if args.scan {
        let (data, _) = display_controller.inventory();
        for row in data { 
            output.output("SCAN_RES", &row); 
        }
        let suites = display_controller.list_suites();
        output.output("SUITES_RES", &suites);
        return Ok(());
    }

    // Run as background service
    if args.daemon {
        return daemon::start_daemon_service(display_controller);
    }

    // Apply specific registry configuration
    if let Some(suite_name) = args.apply_suite {
        display_controller.apply_registry_suite(&suite_name, args.silent)?;
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
            let (inv, _) = display_controller.inventory();
            tasks = inv.iter().map(|r| r.to_task()).collect();
        }
        
        let mut hk = None;
        if args.hotkey { 
            hk = ProfileManager::capture_hotkey(); 
        }

        ProfileManager::create_suite(
            &save_name, &tasks, hk, args.post, args.linkdesktop.is_some(), args.hotkey
        )?;
    } else if !tasks.is_empty() {
        // Apply display tasks immediately
        display_controller.apply(tasks, vec![]);
        std::thread::sleep(std::time::Duration::from_millis(200));
    }

    Ok(())
}