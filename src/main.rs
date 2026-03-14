mod engine;
mod scraper;
mod deployer;
mod daemon;
mod cli;
mod bridge;

use engine::DFEngine;
use scraper::set_dpi_awareness;
use deployer::DeploymentManager;
use cli::{Cli, parse_task};
use bridge::IOBridge;
use clap::Parser;
use anyhow::Result;

fn main() -> Result<()> {
    let args = Cli::parse();
    set_dpi_awareness();
    
    let engine = DFEngine::new();
    let bridge = IOBridge::new(args.daemon);

    if args.scan {
        let (data, _) = engine.inventory();
        for row in data { bridge.output("SCAN_RES", &row); }
        return Ok(());
    }

    if args.daemon {
        return daemon::start_daemon_service(engine);
    }

    if let Some(suite_name) = args.apply_suite {
        engine.apply_registry_suite(&suite_name, args.silent)?;
        return Ok(());
    }

    let mut tasks: Vec<_> = args.tasks.iter().filter_map(|s| parse_task(s)).collect();

    if let Some(save_name) = args.save {
        if tasks.is_empty() {
            let (inv, _) = engine.inventory();
            tasks = inv.iter().map(|r| r.to_task()).collect();
        }
        
        let mut hk = None;
        if args.hotkey { hk = DeploymentManager::capture_hotkey(); }

        DeploymentManager::create_suite(
            &save_name, &tasks, hk, args.post, 
            args.linkdesktop.is_some(), 
            args.linkdesktop.as_deref() == Some("h")
        )?;
        return Ok(());
    }

    if !tasks.is_empty() {
        engine.apply(tasks, vec![]);
    }

    Ok(())
}