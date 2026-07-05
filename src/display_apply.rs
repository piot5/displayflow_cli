//! Display Configuration Application
//! 
//! Applies display settings (resolution, position, primary, rotation)

use std::process::Command;
use std::thread;
use std::time::Duration;
use crate::scraper::DisplayTask;
use crate::scraper::DisplayRow;
use crate::display_snapshot::{stage_registry_setting, commit_registry, gdi_error_to_string, disp_change_to_i32, match_display};
use log::{info, error, debug};
use windows::core::PCWSTR;
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::Foundation::{RECT, BOOL};
use crate::scraper::scan::GdiDevMode;
use std::sync::mpsc::Sender;
use crate::ddc_control::DdcCommand;
use std::iter;

pub fn apply(ddc_tx: &Sender<DdcCommand>, tasks: Vec<DisplayTask>) {
    // Trigger external animation helper (screen_animation.exe) if requested
    let directions: Vec<String> = tasks.iter()
        .filter_map(|t| t.animation.clone())
        .filter(|s| s != "0")
        .collect();

    if !directions.is_empty() {
        thread::spawn(move || {
            let mut cmd = Command::new("screen_animation.exe");
            cmd.arg("--direction");
            for dir in directions { cmd.arg(dir); }
            let _ = cmd.spawn();
        });
        thread::sleep(Duration::from_secs(3));
    }

    let inv = get_active_inventory();
    let snapshot = crate::display_snapshot::take_snapshot();
    let mut sorted_tasks = tasks.clone();

    // Critical: Set primary monitor first. Windows tends to throw windows/icons 
    // to random screens if the primary isn't locked in early.
    sorted_tasks.sort_by_key(|t| !t.is_primary);
    let mut staged_count = 0;

    // Solution #3: Phase 1 - Disable monitors that should be off BEFORE staging new configs
    debug!("Phase 1: Disabling monitors that should be inactive...");
    for task in &sorted_tasks {
        if task.width == 0 && task.height == 0 {
            // This monitor should be disabled
            if let Some(row) = inv.iter().find(|r| match_display(r, &task.query)) {
                let disable_task = DisplayTask {
                    query: task.query.clone(),
                    width: 0,
                    height: 0,
                    freq: 0,
                    x: 0,
                    y: 0,
                    is_primary: false,
                    direction: None,
                    brightness: None,
                    contrast: None,
                    animation: None,
                };
                if stage_config(&row.name_id, &disable_task) {
                    debug!("Staged disable for {}", row.name_id);
                }
                // Small delay between operations to let GDI process
                thread::sleep(Duration::from_millis(100));
            }
        }
    }
    
    // Flush disable operations to hardware
    commit_registry();
    thread::sleep(Duration::from_millis(200));

    // Solution #3: Phase 2 - Now configure monitors that should be active
    debug!("Phase 2: Staging active monitor configurations...");
    for task in &sorted_tasks {
        if task.width > 0 && task.height > 0 {
            // This monitor should be active
            if let Some(row) = inv.iter().find(|r| match_display(r, &task.query)) {
                let was_inactive = snapshot.iter()
                    .find(|(name, _, _)| String::from_utf16_lossy(name).trim_matches(char::from(0)) == row.name_id)
                    .map(|(_, active, _)| !*active)
                    .unwrap_or(false);

                if stage_config(&row.name_id, task) {
                    staged_count += 1;

                    // Fire-and-forget hardware DDC updates.
                    if task.brightness.is_some() || task.contrast.is_some() {
                        if let Some(h_mon) = get_hmonitor_by_name(&row.name_id) {
                            let _ = ddc_tx.send(DdcCommand::Apply {
                                h_monitor: h_mon.0,
                                brightness: task.brightness,
                                contrast: task.contrast,
                                delay: was_inactive,
                            });
                        }
                    }
                }
                // Small delay between operations
                thread::sleep(Duration::from_millis(100));
            }
        }
    }
    
    // Solution #3: Phase 3 - Commit all changes at once
    if staged_count > 0 { 
        info!("Phase 3: Committing {} display configurations to hardware...", staged_count);
        commit_registry(); 
        thread::sleep(Duration::from_millis(300));
    }
}

fn stage_config(name: &str, task: &DisplayTask) -> bool {
    let name_u16: Vec<u16> = name.encode_utf16().chain(iter::once(0)).collect();
    unsafe {
        let mut dm = GdiDevMode::new();
        if EnumDisplaySettingsW(PCWSTR(name_u16.as_ptr()), ENUM_CURRENT_SETTINGS, &mut dm.0).as_bool() {
            // 1. Determine rotation
            let rotation = match task.direction.as_deref() {
                Some("90") | Some("right") => DMDO_90, 
                Some("180") | Some("inverted") => DMDO_180, 
                Some("270") | Some("left") => DMDO_270, 
                _ => DMDO_DEFAULT
            };

            // 2. Adjust dimensions based on rotation
            let (w, h) = if rotation == DMDO_90 || rotation == DMDO_270 { 
                (task.height, task.width) 
            } else { 
                (task.width, task.height) 
            };

            dm.0.dmPelsWidth = w as u32;
            dm.0.dmPelsHeight = h as u32;
            
            // 3. Mark relevant fields
            dm.0.dmFields |= DM_PELSWIDTH | DM_PELSHEIGHT | DM_POSITION | DM_DISPLAYORIENTATION;
            dm.0.Anonymous1.Anonymous2.dmPosition.x = task.x;
            dm.0.Anonymous1.Anonymous2.dmPosition.y = task.y;
            dm.0.Anonymous1.Anonymous2.dmDisplayOrientation = rotation;

            // 4. Optional frequency setting
            if task.freq > 0 {
                dm.0.dmDisplayFrequency = task.freq;
                dm.0.dmFields |= DM_DISPLAYFREQUENCY;
            }

            // 5. Flags for staging
            let mut flags = CDS_UPDATEREGISTRY | CDS_NORESET;
            if task.is_primary { flags |= CDS_SET_PRIMARY; }

            // 6. Disable monitor if resolution is 0/0
            if task.width == 0 && task.height == 0 {
                dm.0.dmPelsWidth = 0;
                dm.0.dmPelsHeight = 0;
                dm.0.dmFields = DM_PELSWIDTH | DM_PELSHEIGHT | DM_POSITION;
                debug!("Staging monitor {} for DISABLE (0x0 resolution)", name);
            }

            // 7. Write to registry without reset
            let res = ChangeDisplaySettingsExW(
                PCWSTR(name_u16.as_ptr()), 
                Some(&dm.0), 
                None, 
                flags, 
                None
            );

            let code = disp_change_to_i32(res);
            if code == 0 || code == 1 {
                info!("Staged display {}: {}x{}@{}Hz at ({},{}) primary={}", 
                    name, w, h, task.freq, task.x, task.y, task.is_primary);
                true
            } else {
                error!("Failed to stage config for {}: {}", name, gdi_error_to_string(code));
                debug!("Task details: width={}, height={}, freq={}, x={}, y={}, primary={}", 
                    task.width, task.height, task.freq, task.x, task.y, task.is_primary);
                false
            }
        } else { 
            error!("Failed to read current display settings for {}", name);
            false 
        }
    }
}

fn get_hmonitor_by_name(target_name: &str) -> Option<HMONITOR> {
    use windows::Win32::Foundation::LPARAM;
    use windows::Win32::Graphics::Gdi::{GetMonitorInfoW, MONITORINFOEXW};
    
    struct EnumCtx { target: String, result: Option<HMONITOR> }
    let mut ctx = EnumCtx { target: target_name.to_string(), result: None };

    unsafe extern "system" fn callback(h_mon: HMONITOR, _: HDC, _: *mut RECT, lparam: LPARAM) -> BOOL {
        let ctx = &mut *(lparam.0 as *mut EnumCtx);
        let mut mi = MONITORINFOEXW::default();
        mi.monitorInfo.cbSize = std::mem::size_of::<MONITORINFOEXW>() as u32;
        if GetMonitorInfoW(h_mon, &mut mi.monitorInfo).as_bool() {
            let device_name = String::from_utf16_lossy(&mi.szDevice).trim_matches(char::from(0)).to_string();
            if device_name == ctx.target {
                ctx.result = Some(h_mon);
                return BOOL(0);  // Match found, stop enumeration.
            }
        }
        BOOL(1)
    }
    unsafe { let _ = EnumDisplayMonitors(None, None, Some(callback), LPARAM(&mut ctx as *mut _ as isize)); }
    ctx.result
}

fn get_active_inventory() -> Vec<DisplayRow> {
    use crate::scraper;
    let snapshot = crate::display_snapshot::take_snapshot();
    for (name_u16, is_active, dm) in &snapshot {
        if !*is_active {
            let mut temp_dm = *dm;
            temp_dm.dmPelsWidth = 1920;
            temp_dm.dmPelsHeight = 1080;
            temp_dm.dmFields = DM_PELSWIDTH | DM_PELSHEIGHT;
            stage_registry_setting(PCWSTR(name_u16.as_ptr()), &temp_dm, 0);
        }
    }
    commit_registry();
    scraper::collect_inventory()
}