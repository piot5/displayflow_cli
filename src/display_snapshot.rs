//! Display Snapshot Management
//! 
//! Handles taking snapshots of current display state and restoring them

use windows::core::PCWSTR;
use windows::Win32::Graphics::Gdi::*;
use crate::scraper::scan::{GdiDevice, GdiDevMode};
use log::{info, warn, error};

/// GDI Error Code Helper
pub fn gdi_error_to_string(code: i32) -> String {
    match code {
        0 => "DISP_CHANGE_SUCCESSFUL: Configuration applied successfully".to_string(),
        1 => "DISP_CHANGE_RESTART: Configuration applied but system restart required".to_string(),
        -1 => "DISP_CHANGE_BADDUALVIEW: Bad dual-view configuration detected. Verify primary is at (0,0)".to_string(),
        -2 => "DISP_CHANGE_BADFLAGS: Invalid flags specified".to_string(),
        -3 => "DISP_CHANGE_BADPARAM: Invalid parameter in DEVMODE".to_string(),
        -4 => "DISP_CHANGE_FAILED: Display driver failed to apply configuration".to_string(),
        -5 => "DISP_CHANGE_BADMODE: Invalid mode or resolution".to_string(),
        -6 => "DISP_CHANGE_NOTUPDATED: Registry updated but display not changed (driver issue)".to_string(),
        _ => format!("UNKNOWN_ERROR (code: {})", code),
    }
}

/// Converts DISP_CHANGE to i32
pub fn disp_change_to_i32(res: DISP_CHANGE) -> i32 {
    res.0 as i32
}

/// Restores a Snapshot of the Display Config
/// Also restores if the process crashes or an apply fails during scan.
pub struct DisplayRestorer {
    pub snapshot: Vec<(Vec<u16>, bool, DEVMODEW)>,
}

impl Drop for DisplayRestorer {
    fn drop(&mut self) {
        for (name_u16, was_active, old_dm) in &self.snapshot {
            let pcw_name = PCWSTR(name_u16.as_ptr());
            let mut reset_dm = *old_dm;
            if !*was_active {
                // Monitor was off before? Set resolution to 0x0 to detach it again.
                reset_dm.dmPelsWidth = 0;
                reset_dm.dmPelsHeight = 0;
                reset_dm.dmFields = DM_PELSWIDTH | DM_PELSHEIGHT | DM_POSITION;
            } else {
                reset_dm.dmFields = DM_PELSWIDTH | DM_PELSHEIGHT | DM_POSITION | DM_DISPLAYORIENTATION;
            }
            // Force global hardware update for all staged changes.
            unsafe {
                let res = ChangeDisplaySettingsExW(pcw_name, Some(&reset_dm), None, CDS_UPDATEREGISTRY | CDS_NORESET, None);
                let code = disp_change_to_i32(res);
                if code != 0 && code != 1 {
                    warn!("Restorer: Failed to stage reset for {:?}: {}", String::from_utf16_lossy(name_u16), gdi_error_to_string(code));
                }
            }
        }
        // Commit der Wiederherstellung
        unsafe { 
            let res = ChangeDisplaySettingsExW(PCWSTR::null(), None, None, CDS_TYPE(0), None);
            let code = disp_change_to_i32(res);
            if code != 0 && code != 1 {
                error!("Restorer: Failed to commit registry changes: {}", gdi_error_to_string(code));
            }
        }
    }
}

/// Stage a registry setting without resetting hardware
pub fn stage_registry_setting(name: PCWSTR, dm: &DEVMODEW, flags: u32) -> bool {
    unsafe {
        let res = ChangeDisplaySettingsExW(name, Some(dm), None, CDS_UPDATEREGISTRY | CDS_NORESET | CDS_TYPE(flags), None);
        let code = disp_change_to_i32(res);
        if code != 0 && code != 1 {
            let device_name = String::from_utf16_lossy(std::slice::from_raw_parts(name.as_ptr(), 32))
                .trim_matches(char::from(0))
                .to_string();
            warn!("stage_registry_setting({}, res={}): {}", device_name, code, gdi_error_to_string(code));
        }
        code == 0 || code == 1
    }
}

/// Commit all pending registry changes to hardware
pub fn commit_registry() {
    unsafe { 
        let res = ChangeDisplaySettingsExW(PCWSTR::null(), None, None, CDS_TYPE(0), None);
        let code = disp_change_to_i32(res);
        if code != 0 && code != 1 {
            error!("commit_registry failed: {}", gdi_error_to_string(code));
            debug!("This typically means Windows GDI rejected the entire configuration. Check:");
            debug!("  1. Is the primary monitor at position (0,0)?");
            debug!("  2. Do any two monitors overlap in position?");
            debug!("  3. Are all resolutions/frequencies valid for the hardware?");
            debug!("  4. Are you using generic/outdated GPU drivers?");
        } else if code == 1 {
            info!("Display configuration applied successfully (restart may be required)");
        }
    } 
}

use log::debug;

/// Take a snapshot of current display state
pub fn take_snapshot() -> Vec<(Vec<u16>, bool, DEVMODEW)> {
    let mut snapshot = Vec::new();
    use std::iter;
    
    unsafe {
        for i in 0..64 {
            let mut device = GdiDevice::new();
            if !EnumDisplayDevicesW(None, i, device.as_mut_ptr(), 0).as_bool() { break; }
            let name: Vec<u16> = device.0.DeviceName.iter().take_while(|&&c| c != 0).cloned().chain(iter::once(0)).collect();
            let mut dm = GdiDevMode::new();
            // Prefer registry settings over live settings to catch pending changes.
            if !EnumDisplaySettingsW(PCWSTR(name.as_ptr()), ENUM_REGISTRY_SETTINGS, &mut dm.0).as_bool() {
                let _ = EnumDisplaySettingsW(PCWSTR(name.as_ptr()), ENUM_CURRENT_SETTINGS, &mut dm.0);
            }
            let active = (device.0.StateFlags & DISPLAY_DEVICE_ATTACHED_TO_DESKTOP) != 0;
            snapshot.push((name, active, dm.0));
        }
    }
    snapshot
}

/// Match display by query string
pub fn match_display(row: &crate::scraper::DisplayRow, query: &str) -> bool {
    let q = query.to_uppercase();
    row.persistent_id.to_string() == q 
        || row.name_id.to_uppercase() == q 
        || row.serial.to_uppercase() == q
        || row.position_instance.split('\\').nth(1).map_or(false, |s| s.to_uppercase() == q)
}