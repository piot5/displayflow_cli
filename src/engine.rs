use std::iter;
use std::thread;
use std::time::Duration;
use std::process::Command;
use std::sync::mpsc::{self, Sender};
use windows::core::PCWSTR;
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::Foundation::{LPARAM, RECT, BOOL};
use winreg::RegKey;
use winreg::enums::HKEY_CURRENT_USER;
use crate::scraper::{self, DisplayTask, DisplayRow, scan::{GdiDevice, GdiDevMode}, ddc};
use crate::cli::parse_task;


/// Guard restores a Snapshot of the Display Config, 
/// but also restores if the process crashes or an apply fails during scan.
pub struct DisplayRestoreGuard {
    pub snapshot: Vec<(Vec<u16>, bool, DEVMODEW)>,
}

impl Drop for DisplayRestoreGuard {
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
		// CDS_NORESET: Just update registry for now. We'll flush the hardware change once at the end.
                let _ = ChangeDisplaySettingsExW(pcw_name, Some(&reset_dm), None, CDS_UPDATEREGISTRY | CDS_NORESET, None);
            }
        }
        // Commit der Wiederherstellung
        unsafe { let _ = ChangeDisplaySettingsExW(PCWSTR::null(), None, None, CDS_TYPE(0), None); }
    }
}

enum DdcCommand {
    Apply {
        h_monitor: isize,
        brightness: Option<u32>,
        contrast: Option<u32>,
        delay: bool,
    },
}

pub struct DFEngine {
    ddc_tx: Sender<DdcCommand>,
}

impl DFEngine {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel::<DdcCommand>();
	// DDC/CI (I2C) operations are notoriously slow and can block.
        // offload to a background thread to prevent UI/CLI lag.
        thread::spawn(move || {
            while let Ok(cmd) = rx.recv() {
                match cmd {
                    DdcCommand::Apply { h_monitor, brightness, contrast, delay } => {
                        if delay {
			    // If a monitor just woke up, the scaler/firmware often 
                            // needs a moment before it starts responding to VCP
                            // Wait for hardware to be ready
                            thread::sleep(Duration::from_millis(800));
                        }
                        let hmon = HMONITOR(h_monitor);
                        ddc::set_monitor_vcp(hmon, brightness, contrast);
                    }
                }
            }
        });

        Self { ddc_tx: tx }
    }

    pub fn inventory(&self) -> (Vec<DisplayRow>, DisplayRestoreGuard) {
        let snapshot = self.take_snapshot();
// Activate all to ensure edid is parsed .
        let fallbacks = [(1920, 1080), (1280, 720)];

        for (name_u16, is_active, dm) in &snapshot {
            if !*is_active {
// Stage a dummy resolution in the registry so the scraper can see the device.
                for (w, h) in fallbacks {
                    let mut temp_dm = *dm;
                    temp_dm.dmPelsWidth = w;
                    temp_dm.dmPelsHeight = h;
                    temp_dm.dmFields = DM_PELSWIDTH | DM_PELSHEIGHT;
                    if self.stage_registry_setting(PCWSTR(name_u16.as_ptr()), &temp_dm, 0) { break; }
                }
            }
        }
        self.commit_registry();
        let guard = DisplayRestoreGuard { snapshot };
        (scraper::collect_inventory(), guard)
    }

    pub fn apply(&self, tasks: Vec<DisplayTask>, _clones: Vec<(String, String)>) {
// Trigger external animation helper (screen_animation.exe) if requested (it surpresses win intern animation and blends to other screen)       
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

        let (inv, snapshot) = (self.get_active_inventory_raw(), self.take_snapshot());
        let mut sorted_tasks = tasks.clone();

// Critical: Set primary monitor first. Windows tends to throw windows/icons 
// to random screens if the primary isn't locked in early.
        sorted_tasks.sort_by_key(|t| !t.is_primary);
        let mut staged_count = 0;

        for task in &sorted_tasks {
            if let Some(row) = inv.iter().find(|r| self.match_display(r, &task.query)) {
                let was_inactive = snapshot.iter()
                    .find(|(name, _, _)| String::from_utf16_lossy(name).trim_matches(char::from(0)) == row.name_id)
                    .map(|(_, active, _)| !*active)
                    .unwrap_or(false);

                if self.stage_config(&row.name_id, task) {
                    staged_count += 1;

		 // Fire-and-forget hardware DDC updates.
                    if task.brightness.is_some() || task.contrast.is_some() {
                        if let Some(h_mon) = self.find_hmonitor_by_name(&row.name_id) {
                            let _ = self.ddc_tx.send(DdcCommand::Apply {
                                h_monitor: h_mon.0,
                                brightness: task.brightness,
                                contrast: task.contrast,
                                delay: was_inactive,
                            });
                        }
                    }
                }
            }
        }
        if staged_count > 0 { self.commit_registry(); }
    }
/// Resolves GDI device names (\\.\DISPLAY1) to HMONITOR handles for DDC/CI access.
    fn find_hmonitor_by_name(&self, target_name: &str) -> Option<HMONITOR> {
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

    pub fn list_suites(&self) -> Vec<String> {
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        if let Ok(key) = hkcu.open_subkey(r"Software\DisplayFlow\Suites") {
            key.enum_values().map_while(std::result::Result::ok).map(|(name, _)| name).collect()
        } else { Vec::new() }
    }

    pub fn apply_registry_suite(&self, name: &str, silent: bool) -> anyhow::Result<()> {
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let key = hkcu.open_subkey(r"Software\DisplayFlow\Suites")?;
        let config: String = key.get_value(name)?;
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
            self.apply(tasks, vec![]);
            if !silent {
// Run post-action (e.g.  Steam, Playnite,Exel... whatever you need ).
                if let Some(cmd) = post_cmd {
                    let _ = std::process::Command::new("cmd").args(["/C", &cmd]).spawn();
                }
            }
        }
        Ok(())
    }

/// To get reliable HW data from inactive screens, 
/// we briefly enable them in the registry, scrape, then revert.

    fn get_active_inventory_raw(&self) -> Vec<DisplayRow> {
        let snapshot = self.take_snapshot();
        for (name_u16, is_active, dm) in &snapshot {
            if !*is_active {
                let mut temp_dm = *dm;
                temp_dm.dmPelsWidth = 1920;
                temp_dm.dmPelsHeight = 1080;
                temp_dm.dmFields = DM_PELSWIDTH | DM_PELSHEIGHT;
                self.stage_registry_setting(PCWSTR(name_u16.as_ptr()), &temp_dm, 0);
            }
        }
        self.commit_registry();
        let data = scraper::collect_inventory();

	// Cleanup: Revert to previous state.
        for (name_u16, was_active, old_dm) in snapshot {
            if !was_active {
                let mut reset_dm = old_dm;
                reset_dm.dmPelsWidth = 0;
                reset_dm.dmPelsHeight = 0;
                reset_dm.dmFields = DM_PELSWIDTH | DM_PELSHEIGHT;
                self.stage_registry_setting(PCWSTR(name_u16.as_ptr()), &reset_dm, 0);
            }
        }
        self.commit_registry();
        data
    }

    fn take_snapshot(&self) -> Vec<(Vec<u16>, bool, DEVMODEW)> {
        let mut snapshot = Vec::new();
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

    fn stage_registry_setting(&self, name: PCWSTR, dm: &DEVMODEW, flags: u32) -> bool {
        unsafe {
            let res = ChangeDisplaySettingsExW(name, Some(dm), None, CDS_UPDATEREGISTRY | CDS_NORESET | CDS_TYPE(flags), None);
            res == DISP_CHANGE_SUCCESSFUL || res == DISP_CHANGE_RESTART
        }
    }

    fn match_display(&self, row: &DisplayRow, query: &str) -> bool {
        let q = query.to_uppercase();
        row.persistent_id.to_string() == q 
            || row.name_id.to_uppercase() == q 
            || row.serial.to_uppercase() == q
            || row.position_instance.split('\\').nth(1).map_or(false, |s| s.to_uppercase() == q)
    }

    fn commit_registry(&self) {
// Calling with NULL name triggers a hardware flush for all pending registry changes. 
        unsafe { let _ = ChangeDisplaySettingsExW(PCWSTR::null(), None, None, CDS_TYPE(0), None); } 
    }

   fn stage_config(&self, name: &str, task: &DisplayTask) -> bool {
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
            // Note: We must mark dmFields to ensure Windows doesn't ignore the 0 values.
            if task.width == 0 && task.height == 0 {
                dm.0.dmPelsWidth = 0;
                dm.0.dmPelsHeight = 0;
                dm.0.dmFields = DM_PELSWIDTH | DM_PELSHEIGHT | DM_POSITION;
            }

            // 7. Write to registry without reset
            ChangeDisplaySettingsExW(
                PCWSTR(name_u16.as_ptr()), 
                Some(&dm.0), 
                None, 
                flags, 
                None
            ) == DISP_CHANGE_SUCCESSFUL
        } else { 
            false 
        }
    }
}
}