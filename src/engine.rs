use std::iter;
use chrono::Local;
use windows::core::PCWSTR;
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::Devices::Display::{SetDisplayConfig, SDC_APPLY, SDC_TOPOLOGY_EXTEND, SDC_TOPOLOGY_CLONE};
use crate::scraper::{self, DisplayTask, DisplayRow, scan::{GdiDevice, GdiDevMode}};

pub struct DFEngine; 
impl DFEngine {
    pub fn new() -> Self { Self }

    pub fn log(&self, tag: &str, msg: &str) {
        let ts = Local::now().format("%Y%m%dT%H%M%S");
        println!("[{}] [{}] {}", ts, tag, msg);
    }

    pub fn full_scan_discovery(&self) -> Vec<DisplayRow> {
        self.log("INIT", "CCD_SCAN_START");
        unsafe {
            let _ = SetDisplayConfig(None, None, SDC_APPLY | SDC_TOPOLOGY_EXTEND);
            let mut snapshot: Vec<(Vec<u16>, bool, DEVMODEW)> = Vec::new();
            let fallbacks = [(1920, 1080), (1280, 720), (1024, 768), (800, 600)];

            for i in 0..64 {
                let mut device = GdiDevice::new();
                if !EnumDisplayDevicesW(None, i, device.as_mut_ptr(), 0).as_bool() { break; }

                let name_u16: Vec<u16> = device.0.DeviceName.iter()
                    .take_while(|&&c| c != 0)
                    .cloned()
                    .chain(iter::once(0))
                    .collect();
                
                let pcw_name = PCWSTR(name_u16.as_ptr());
                let mut dm = GdiDevMode::new();

                if !EnumDisplaySettingsW(pcw_name, ENUM_REGISTRY_SETTINGS, &mut dm.0).as_bool() {
                    let _ = EnumDisplaySettingsW(pcw_name, ENUM_CURRENT_SETTINGS, &mut dm.0);
                }

                let is_active = (device.0.StateFlags & DISPLAY_DEVICE_ATTACHED_TO_DESKTOP) != 0;
                snapshot.push((name_u16.clone(), is_active, dm.0));

                if !is_active {
                    for (w, h) in fallbacks {
                        let mut temp_dm = dm.0;
                        temp_dm.dmPelsWidth = w;
                        temp_dm.dmPelsHeight = h;
                        temp_dm.dmFields = DM_PELSWIDTH | DM_PELSHEIGHT;
                        if self.stage_registry_setting(pcw_name, &temp_dm, 0) { break; }
                    }
                }
            }

            self.commit_registry();
            let inventory = scraper::collect_inventory();

            for (name_vec, was_active, old_dm) in snapshot {
                let pcw_name = PCWSTR(name_vec.as_ptr());
                let mut reset_dm = old_dm;
                if !was_active {
                    reset_dm.dmPelsWidth = 0;
                    reset_dm.dmPelsHeight = 0;
                }
                reset_dm.dmFields = DM_PELSWIDTH | DM_PELSHEIGHT;
                self.stage_registry_setting(pcw_name, &reset_dm, 0);
            }

            self.commit_registry();
            inventory
        }
    }

    fn stage_registry_setting(&self, name: PCWSTR, dm: &DEVMODEW, flags: u32) -> bool {
        unsafe {
            let res = ChangeDisplaySettingsExW(name, Some(dm), None, CDS_UPDATEREGISTRY | CDS_NORESET | CDS_TYPE(flags), None);
            res == DISP_CHANGE_SUCCESSFUL || res == DISP_CHANGE_RESTART
        }
    }

    pub fn atomic_deploy(&self, tasks: Vec<DisplayTask>, clones: Vec<(String, String)>) {
        self.log("SYNC", "ATOMIC_COMMIT_START");
        let inv = self.full_scan_discovery();

        if !clones.is_empty() {
            unsafe { let _ = SetDisplayConfig(None, None, SDC_APPLY | SDC_TOPOLOGY_CLONE); }
        }

        let mut sorted_tasks = tasks.clone();
        sorted_tasks.sort_by_key(|t| !t.is_primary);

        let mut staged_count = 0;
        for task in &sorted_tasks {
            if let Some(dev) = inv.iter().find(|r| self.match_display(r, &task.query)) {
                if self.stage_config(&dev.name_id, task) {
                    staged_count += 1;
                }
            }
        }

        if staged_count > 0 || !clones.is_empty() {
            self.commit_registry();
            self.log("OK", "ATOMIC_COMMIT_SUCCESS");
        }
    }

    fn match_display(&self, row: &DisplayRow, query: &str) -> bool {
        let q = query.to_uppercase();
        row.persistent_id.to_string() == q 
            || row.name_id.to_uppercase() == q 
            || row.position_instance.split('\\').nth(1).map_or(false, |s| s.to_uppercase() == q)
    }

    fn commit_registry(&self) {
        unsafe { let _ = ChangeDisplaySettingsExW(None, None, None, CDS_TYPE(0), None); }
    }

    fn stage_config(&self, name: &str, task: &DisplayTask) -> bool {
        let name_u16: Vec<u16> = name.encode_utf16().chain(iter::once(0)).collect();
        unsafe {
            let mut dm = GdiDevMode::new();
            if EnumDisplaySettingsW(PCWSTR(name_u16.as_ptr()), ENUM_CURRENT_SETTINGS, &mut dm.0).as_bool() {
                let rotation = match task.direction.as_deref() {
                    Some("90") => DMDO_90,
                    Some("180") => DMDO_180,
                    Some("270") => DMDO_270,
                    _ => DMDO_DEFAULT
                };

                let (w, h) = if rotation == DMDO_90 || rotation == DMDO_270 {
                    (task.height, task.width)
                } else {
                    (task.width, task.height)
                };

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
            } else {
                false
            }
        }
    }
}