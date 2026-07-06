//! Apply display tasks using df_displmgr (topology-aware)

use log::{info, error, debug};
use std::thread;
use std::time::Duration;
use tokio::runtime::Runtime;

use df_displmgr::{activate_with_topology_restore, DisplayResult, ActivationPlan, DisplayId as DfDisplayId};
use df_displmgr::types::{Point2D, Extent2D, DisplayRotation};
use df_displmgr::NativeTopology;

use crate::scraper::{DisplayTask, DisplayRow};
use crate::ddc_control::DdcControl;

/// Resolve a target id for a DisplayRow using NativeTopology outputs.
/// This function tries several heuristics: exact monitor_name match, connector id match,
/// hardware UUID match, or (as fallback) numeric parse of identity.id.
fn resolve_target_id_by_row(row: &DisplayRow) -> Option<u32> {
    if let Ok(mut topo) = NativeTopology::acquire() {
        let outputs = topo.get_outputs();
        for o in outputs {
            let name = o.identity.monitor_name.trim();
            // Prefer exact monitor name
            if !name.is_empty() && name == row.name_id {
                // If the id can be parsed to u32, return it.
                if let Ok(n) = o.identity.id.0.parse::<u32>() { return Some(n); }
                // If connector_id contains a numeric id, attempt parse
                if let Ok(n) = o.identity.connector_id.0.parse::<u32>() { return Some(n); }
            }
            // Try hardware uuid
            if let Some(hw) = &o.identity.hardware_uuid {
                if hw == &row.serial && !hw.is_empty() {
                    if let Ok(n) = o.identity.id.0.parse::<u32>() { return Some(n); }
                }
            }
        }
    }
    None
}

fn map_rotation(direction: Option<&str>) -> Option<DisplayRotation> {
    match direction {
        Some("90") | Some("right") => Some(DisplayRotation::Rotate90),
        Some("180") | Some("inverted") => Some(DisplayRotation::Rotate180),
        Some("270") | Some("left") => Some(DisplayRotation::Rotate270),
        _ => None,
    }
}

/// Main apply function: builds ActivationPlan per task and calls the df_displmgr activation path.
/// This blocks on an internal tokio runtime to call async API.
pub fn apply(ddc_ctl: &DdcControl, tasks: Vec<DisplayTask>) {
    // Trigger external animation helper if requested
    let mut directions: Vec<String> = tasks.iter()
        .filter_map(|t| t.animation.clone())
        .filter(|s| s != "0")
        .collect();

    if !directions.is_empty() {
        thread::spawn(move || {
            let mut cmd = std::process::Command::new("screen_animation.exe");
            cmd.arg("--direction");
            for dir in directions { cmd.arg(dir); }
            let _ = cmd.spawn();
        });
        thread::sleep(Duration::from_secs(2));
    }

    // Inventory - use existing helpers
    let inv = crate::display_apply::get_active_inventory();

    // Create a tokio runtime to call df_displmgr async APIs
    let rt = Runtime::new().expect("failed to create tokio runtime");

    for task in tasks {
        if let Some(row) = inv.iter().find(|r| crate::display_snapshot::match_display(r, &task.query)) {
            if let Some(target_id) = resolve_target_id_by_row(row) {
                let plan = ActivationPlan {
                    position: Some(Point2D { x: task.x, y: task.y }),
                    resolution: Some(Extent2D { width: task.width as u32, height: task.height as u32 }),
                    rotation: map_rotation(task.direction.as_deref()),
                };

                debug!("Activating target_id={} plan={{x:{},y:{},w:{},h:{}}}", target_id, task.x, task.y, task.width, task.height);

                let activate_res = rt.block_on(async {
                    activate_with_topology_restore(target_id, &plan).await
                });

                match activate_res {
                    Ok(_) => info!("Activated {} (query={})", row.name_id, task.query),
                    Err(e) => error!("Activation failed for {}: {:?}", task.query, e),
                }

                // Fire-and-forget DDC updates via ddc_ctl (df_ddc-based)
                if task.brightness.is_some() || task.contrast.is_some() {
                    if let Some(idx) = ddc_ctl.find_device_index_by_name(&row.name_id) {
                        ddc_ctl.apply_by_index(idx, task.brightness, task.contrast, false);
                    }
                }

            } else {
                error!("Could not resolve target id for '{}' (row: {:?})", task.query, row.name_id);
            }
        } else {
            error!("No inventory match for '{}'", task.query);
        }
    }
}
