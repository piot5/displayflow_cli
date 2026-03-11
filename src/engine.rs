use std::{mem, iter};
use chrono::Local;
use windows::core::PCWSTR;
use windows::Win32::Graphics::Gdi::{
    EnumDisplaySettingsW, ChangeDisplaySettingsExW, DEVMODEW, ENUM_CURRENT_SETTINGS,
    CDS_UPDATEREGISTRY, CDS_NORESET, CDS_SET_PRIMARY, DM_PELSWIDTH, DM_PELSHEIGHT, 
    DM_POSITION, DM_DISPLAYORIENTATION, DM_DISPLAYFREQUENCY, CDS_TYPE, DISP_CHANGE_SUCCESSFUL,
    DMDO_DEFAULT, DMDO_90, DMDO_180, DMDO_270, DISPLAY_DEVICEW, EnumDisplayDevicesW,
    ENUM_REGISTRY_SETTINGS, DISP_CHANGE_RESTART
};
use windows::Win32::Devices::Display::{SetDisplayConfig, SDC_APPLY, SDC_TOPOLOGY_EXTEND, SDC_TOPOLOGY_CLONE};
use crate::scraper::{self, DisplayTask, DisplayRow};

pub struct DFEngine; 

impl DFEngine {
    pub fn new() -> Self {
        Self
    }

    pub fn log(&self, tag: &str, msg: &str) {
        let ts = Local::now().format("%Y%m%dT%H%M%S");
        println!("[{}] [{}] {}", ts, tag, msg);
    }

    
    pub fn full_scan_discovery(&self) -> Vec<DisplayRow> {
        self.log("INIT", "CCD_SCAN_START");
        unsafe {
            let _ = SetDisplayConfig(None, None, SDC_APPLY | SDC_TOPOLOGY_EXTEND);
            let mut snapshot = Vec::new();
            let fallbacks = [(1920, 1080), (1280, 720), (1024, 768), (800, 600)];

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
                    for (w, h) in fallbacks {
                        let mut temp_dm = dm.clone();
                        temp_dm.dmPelsWidth = w;
                        temp_dm.dmPelsHeight = h;
                        temp_dm.dmFields = DM_PELSWIDTH | DM_PELSHEIGHT;

                        let res = ChangeDisplaySettingsExW(pcw_name, Some(&temp_dm), None, CDS_UPDATEREGISTRY | CDS_NORESET, None);
                        if res == DISP_CHANGE_SUCCESSFUL || res == DISP_CHANGE_RESTART { break; }
                    }
                }
            }

            self.commit_registry();
            let inventory = scraper::collect_inventory();

            for (name_ptr, was_active, old_dm) in snapshot {
                let pcw_name = PCWSTR(name_ptr.as_ptr());
                let mut reset_dm = old_dm.clone();
                if !was_active {
                    reset_dm.dmPelsWidth = 0;
                    reset_dm.dmPelsHeight = 0;
                }
                reset_dm.dmFields = DM_PELSWIDTH | DM_PELSHEIGHT;
                ChangeDisplaySettingsExW(pcw_name, Some(&reset_dm), None, CDS_UPDATEREGISTRY | CDS_NORESET, None);
            }
            
            self.commit_registry();
            inventory
        }
    }

    pub fn execute_clone(&self, src_query: &str, tgt_query: &str) {
        self.log("SYNC", "TOPOLOGY_CLONE_INIT");
        let inv = self.full_scan_discovery();
        
        let find_node = |q: &str| inv.iter().find(|r| self.match_display(r, q));

        if find_node(src_query).is_some() && find_node(tgt_query).is_some() {
            unsafe {
                if SetDisplayConfig(None, None, SDC_APPLY | SDC_TOPOLOGY_CLONE) == 0 {
                    self.log("OK", "CLONE_APPLIED");
                } else {
                    self.log("FAIL", "CLONE_ERR_01");
                }
            }
        } else {
            self.log("FAIL", "NODE_NOT_FOUND");
        }
    }

    pub fn execute_integrated(&self, tasks: Vec<DisplayTask>) {
        let inv = self.full_scan_discovery();
        let mut sorted = tasks.clone();
        sorted.sort_by_key(|t| !t.is_primary);
        
        let mut staged = 0;
        for task in &sorted {
            if let Some(dev) = inv.iter().find(|r| self.match_display(r, &task.query)) {
                if self.stage_config(&dev.name_id, task) {
                    staged += 1;
                }
            } else {
                self.log("FAIL", &format!("NODE_MISSING: {}", task.query));
            }
        }

        if staged > 0 {
            
            self.commit_registry(); 
            self.log("SYNC", "LAYOUT_COMMIT_SUCCESS");
        }
    }

    fn match_display(&self, row: &DisplayRow, query: &str) -> bool {
        let q = query.to_uppercase();
        row.persistent_id.to_string() == q || 
        row.name_id.to_uppercase() == q ||
        row.position_instance.split('\\').nth(1).map_or(false, |s| s.to_uppercase() == q)
    }

    fn commit_registry(&self) {
        unsafe { ChangeDisplaySettingsExW(None, None, None, CDS_TYPE(0), None); }
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

                let (w, h) = if rotation == DMDO_90 || rotation == DMDO_270 {
                    (task.height, task.width)
                } else {
                    (task.width, task.height)
                };

                dm.dmPelsWidth = w as u32;
                dm.dmPelsHeight = h as u32;
                dm.dmFields |= DM_PELSWIDTH | DM_PELSHEIGHT | DM_POSITION | DM_DISPLAYORIENTATION;
                dm.Anonymous1.Anonymous2.dmPosition.x = task.x;
                dm.Anonymous1.Anonymous2.dmPosition.y = task.y;
                dm.Anonymous1.Anonymous2.dmDisplayOrientation = rotation;

                if task.freq > 0 {
                    dm.dmDisplayFrequency = task.freq;
                    dm.dmFields |= DM_DISPLAYFREQUENCY;
                }

                let mut flags = CDS_UPDATEREGISTRY | CDS_NORESET;
                if task.is_primary { flags |= CDS_SET_PRIMARY; }

                ChangeDisplaySettingsExW(PCWSTR(name_u16.as_ptr()), Some(&dm), None, flags, None) == DISP_CHANGE_SUCCESSFUL
            } else {
                false
            }
        }
    }
}