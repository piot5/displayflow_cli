use std::{mem, iter};
use chrono::Local;
use windows::core::PCWSTR;
use windows::Win32::Graphics::Gdi::{
    EnumDisplaySettingsW, ChangeDisplaySettingsExW, DEVMODEW, ENUM_CURRENT_SETTINGS,
    CDS_UPDATEREGISTRY, CDS_NORESET, CDS_SET_PRIMARY, DM_PELSWIDTH, DM_PELSHEIGHT, 
    DM_POSITION, DM_DISPLAYORIENTATION, CDS_TYPE, DISP_CHANGE_SUCCESSFUL,
    DMDO_DEFAULT, DMDO_90, DMDO_180, DMDO_270, DISPLAY_DEVICEW, EnumDisplayDevicesW,
    ENUM_REGISTRY_SETTINGS, DISP_CHANGE_RESTART
};
use windows::Win32::Devices::Display::{SetDisplayConfig, SDC_APPLY, SDC_TOPOLOGY_EXTEND};
use crate::scraper::{self, DisplayTask, DisplayRow};

pub struct DFEngine; 

impl DFEngine {
    pub fn new() -> Self {
        Self
    }

    pub fn log(&self, status: &str, msg: &str) {
        let ts = Local::now().format("%H:%M:%S");
        let output = format!("[{}] [{:<7}] {}\n", ts, status, msg);
        // Nur noch Konsolenausgabe, keine Dateioperationen mehr
        print!("{}", output);
    }

    /// Performs a full scan and briefly reactivates all ports 
    pub fn full_scan_discovery(&self) -> Vec<DisplayRow> {
        self.log("FORCE", "Initializing CCD Safe-Handshake...");
        
        unsafe {
            let _ = SetDisplayConfig(None, None, SDC_APPLY | SDC_TOPOLOGY_EXTEND);
            let mut snapshot = Vec::new();
            let fallbacks = vec![(1920, 1080), (1280, 720), (1024, 768), (800, 600)];

            for i in 0..64 {
                let mut device: DISPLAY_DEVICEW = mem::zeroed();
                device.cb = mem::size_of::<DISPLAY_DEVICEW>() as u32;
                if !EnumDisplayDevicesW(None, i, &mut device, 0).as_bool() { break; }

                let name_u16: Vec<u16> = device.DeviceName.iter().take_while(|&&c| c != 0).cloned().collect();
                let pcw_name = PCWSTR(name_u16.as_ptr());
                
                let mut dm: DEVMODEW = mem::zeroed();
                dm.dmSize = mem::size_of::<DEVMODEW>() as u16;

                if !EnumDisplaySettingsW(pcw_name, ENUM_REGISTRY_SETTINGS, &mut dm).as_bool() {
                    EnumDisplaySettingsW(pcw_name, ENUM_CURRENT_SETTINGS, &mut dm);
                }

                let is_active = (device.StateFlags & 0x1) != 0;
                snapshot.push((name_u16.clone(), is_active, dm.clone()));

                if !is_active {
                    for (w, h) in &fallbacks {
                        let mut temp_dm = dm.clone();
                        temp_dm.dmPelsWidth = *w;
                        temp_dm.dmPelsHeight = *h;
                        temp_dm.dmFields = DM_PELSWIDTH | DM_PELSHEIGHT;

                        let res = ChangeDisplaySettingsExW(pcw_name, Some(&temp_dm), None, CDS_UPDATEREGISTRY | CDS_NORESET, None);
                        if res == DISP_CHANGE_SUCCESSFUL || res == DISP_CHANGE_RESTART {
                            break;
                        }
                    }
                }
            }

            ChangeDisplaySettingsExW(None, None, None, CDS_TYPE(0), None);
            let inventory = scraper::collect_inventory();

            for (name_ptr, was_active, old_dm) in snapshot {
                let pcw_name = PCWSTR(name_ptr.as_ptr());
                if !was_active {
                    let mut off_dm = old_dm.clone();
                    off_dm.dmPelsWidth = 0;
                    off_dm.dmPelsHeight = 0;
                    off_dm.dmFields = DM_PELSWIDTH | DM_PELSHEIGHT;
                    ChangeDisplaySettingsExW(pcw_name, Some(&off_dm), None, CDS_UPDATEREGISTRY | CDS_NORESET, None);
                } else {
                    ChangeDisplaySettingsExW(pcw_name, Some(&old_dm), None, CDS_UPDATEREGISTRY | CDS_NORESET, None);
                }
            }
            
            ChangeDisplaySettingsExW(None, None, None, CDS_TYPE(0), None);
            inventory
        }
    }

    pub fn execute_integrated(&self, tasks: Vec<DisplayTask>) {
        let inventory = self.full_scan_discovery();
        let mut sorted = tasks.clone();
        sorted.sort_by_key(|t| !t.is_primary);
        
        let mut staged_count = 0;
        for task in &sorted {
            let query = task.query.to_uppercase();
            let target = inventory.iter().find(|r| {
                let id_match = r.persistent_id.to_string() == query;
                let clean_hwid = r.position_instance.split('\\').nth(1).unwrap_or("").to_uppercase();
                let hw_match = clean_hwid == query;
                let name_match = r.name_id.to_uppercase() == query;
                id_match || hw_match || name_match
            });

            if let Some(dev) = target {
                if self.stage_config(&dev.name_id, task) {
                    self.log("STAGE", &format!("Target Ready: {} ({})", task.query, dev.name_id));
                    staged_count += 1;
                }
            } else {
                self.log("WARN", &format!("Target '{}' was not found in the system.", task.query));
            }
        }

        if staged_count > 0 {
            unsafe { ChangeDisplaySettingsExW(None, None, None, CDS_TYPE(0), None); }
            self.log("COMMIT", "New display layout applied successfully.");
        }
    }

    fn stage_config(&self, name: &str, task: &DisplayTask) -> bool {
        let name_u16: Vec<u16> = name.encode_utf16().chain(iter::once(0)).collect();
        unsafe {
            let mut dm: DEVMODEW = mem::zeroed();
            dm.dmSize = mem::size_of::<DEVMODEW>() as u16;
            
            if EnumDisplaySettingsW(PCWSTR(name_u16.as_ptr()), ENUM_CURRENT_SETTINGS, &mut dm).as_bool() {
                let rotation = match task.direction.as_deref() {
                    Some("90") => DMDO_90,
                    Some("180") => DMDO_180,
                    Some("270") => DMDO_270,
                    _ => DMDO_DEFAULT,
                };

                if rotation == DMDO_90 || rotation == DMDO_270 {
                    dm.dmPelsWidth = task.height as u32;
                    dm.dmPelsHeight = task.width as u32;
                } else {
                    dm.dmPelsWidth = task.width as u32;
                    dm.dmPelsHeight = task.height as u32;
                }

                dm.dmFields |= DM_PELSWIDTH | DM_PELSHEIGHT | DM_POSITION | DM_DISPLAYORIENTATION;
                dm.Anonymous1.Anonymous2.dmPosition.x = task.x;
                dm.Anonymous1.Anonymous2.dmPosition.y = task.y;
                dm.Anonymous1.Anonymous2.dmDisplayOrientation = rotation;

                let mut flags = CDS_UPDATEREGISTRY | CDS_NORESET;
                if task.is_primary { flags |= CDS_SET_PRIMARY; }

                let res = ChangeDisplaySettingsExW(PCWSTR(name_u16.as_ptr()), Some(&dm), None, flags, None);
                return res == DISP_CHANGE_SUCCESSFUL;
            }
        }
        false
    }
}