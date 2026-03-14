use std::iter;
use windows::core::PCWSTR;
use windows::Win32::Graphics::Gdi::*;
use winreg::RegKey;
use winreg::enums::HKEY_CURRENT_USER;
use crate::scraper::{self, DisplayTask, DisplayRow, scan::{GdiDevice, GdiDevMode}};
use crate::cli::parse_task;

pub struct DisplayRestoreGuard {
    pub snapshot: Vec<(Vec<u16>, bool, DEVMODEW)>,
}

impl Drop for DisplayRestoreGuard {
    fn drop(&mut self) {
        for (name_u16, was_active, old_dm) in &self.snapshot {
            let pcw_name = PCWSTR(name_u16.as_ptr());
            let mut reset_dm = *old_dm;
            if !*was_active {
                reset_dm.dmPelsWidth = 0;
                reset_dm.dmPelsHeight = 0;
                reset_dm.dmFields = DM_PELSWIDTH | DM_PELSHEIGHT | DM_POSITION;
            } else {
                reset_dm.dmFields = DM_PELSWIDTH | DM_PELSHEIGHT | DM_POSITION | DM_DISPLAYORIENTATION;
            }
            unsafe {
                let _ = ChangeDisplaySettingsExW(pcw_name, Some(&reset_dm), None, CDS_UPDATEREGISTRY | CDS_NORESET, None);
            }
        }
        unsafe { let _ = ChangeDisplaySettingsExW(None, None, None, CDS_TYPE(0), None); }
    }
}

pub struct DFEngine; 
impl DFEngine {
    pub fn new() -> Self { Self }

    pub fn inventory(&self) -> (Vec<DisplayRow>, DisplayRestoreGuard) {
        let snapshot = self.take_snapshot();
        let fallbacks = [(1920, 1080), (1280, 720)];

        for (name_u16, is_active, dm) in &snapshot {
            if !*is_active {
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
        let inv = self.get_active_inventory();
        let mut sorted_tasks = tasks.clone();
        sorted_tasks.sort_by_key(|t| !t.is_primary);
        let mut staged_count = 0;
        for task in &sorted_tasks {
            if let Some(dev) = inv.iter().find(|r| self.match_display(r, &task.query)) {
                if self.stage_config(&dev.name_id, task) { staged_count += 1; }
            }
        }
        if staged_count > 0 { self.commit_registry(); }
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
                if let Some(cmd) = post_cmd {
                    let _ = std::process::Command::new("cmd").args(["/C", &cmd]).spawn();
                }
            }
        }
        Ok(())
    }

    fn get_active_inventory(&self) -> Vec<DisplayRow> {
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
        row.persistent_id.to_string() == q || row.name_id.to_uppercase() == q 
            || row.position_instance.split('\\').nth(1).map_or(false, |s| s.to_uppercase() == q)
    }

    fn commit_registry(&self) { unsafe { let _ = ChangeDisplaySettingsExW(None, None, None, CDS_TYPE(0), None); } }

    fn stage_config(&self, name: &str, task: &DisplayTask) -> bool {
        let name_u16: Vec<u16> = name.encode_utf16().chain(iter::once(0)).collect();
        unsafe {
            let mut dm = GdiDevMode::new();
            if EnumDisplaySettingsW(PCWSTR(name_u16.as_ptr()), ENUM_CURRENT_SETTINGS, &mut dm.0).as_bool() {
                let rotation = match task.direction.as_deref() {
                    Some("90") => DMDO_90, Some("180") => DMDO_180, Some("270") => DMDO_270, _ => DMDO_DEFAULT
                };
                let (w, h) = if rotation == DMDO_90 || rotation == DMDO_270 { (task.height, task.width) } else { (task.width, task.height) };
                dm.0.dmPelsWidth = w as u32;
                dm.0.dmPelsHeight = h as u32;
                dm.0.dmFields |= DM_PELSWIDTH | DM_PELSHEIGHT | DM_POSITION | DM_DISPLAYORIENTATION;
                dm.0.Anonymous1.Anonymous2.dmPosition.x = task.x;
                dm.0.Anonymous1.Anonymous2.dmPosition.y = task.y;
                dm.0.Anonymous1.Anonymous2.dmDisplayOrientation = rotation;
                if task.freq > 0 {
                    dm.0.dmDisplayFrequency = task.freq;
                    dm.0.dmFields |= DM_DISPLAYFREQUENCY;
                }
                let mut flags = CDS_UPDATEREGISTRY | CDS_NORESET;
                if task.is_primary { flags |= CDS_SET_PRIMARY; }
                ChangeDisplaySettingsExW(PCWSTR(name_u16.as_ptr()), Some(&dm.0), None, flags, None) == DISP_CHANGE_SUCCESSFUL
            } else { false }
        }
    }
}