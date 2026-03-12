use std::{mem, iter, thread, time::Duration};
use chrono::Local;
use windows::core::PCWSTR;
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::Devices::Display::{SetDisplayConfig, SDC_APPLY, SDC_TOPOLOGY_EXTEND, SDC_TOPOLOGY_INTERNAL, SDC_ALLOW_CHANGES};
use crate::scraper::{self, DisplayTask, DisplayRow};

pub struct DFEngine;

impl DFEngine {
    pub fn new() -> Self { Self }

    pub fn log(&self, tag: &str, msg: &str) {
        let ts = Local::now().format("%Y%m%dT%H%M%S");
        println!("[{}] [{}] {}", ts, tag, msg);
    }

    pub fn atomic_deploy(&self, tasks: Vec<DisplayTask>, _clones: Vec<(String, String)>) {
        // 1. Nutze die bewährte Discovery-Funktion um das System vorzubereiten
        let inv = self.full_scan_discovery();

        let (to_activate, to_deactivate): (Vec<_>, Vec<_>) = tasks.into_iter()
            .partition(|t| t.width > 0 && t.height > 0);

        // PHASE A: Detach (BNQ aus)
        for task in &to_deactivate {
            if let Some(dev) = inv.iter().find(|r| self.match_display(r, &task.query)) {
                self.force_set_registry(&dev.name_id, 0, 0, 0, 0, false);
            }
        }
        self.commit_registry();

        // PHASE B: Wunsch-Layout (TCL auf 1080p)
        let mut sorted_active = to_activate;
        sorted_active.sort_by_key(|t| !t.is_primary);

        for task in &sorted_active {
            if let Some(dev) = inv.iter().find(|r| self.match_display(r, &task.query)) {
                self.force_set_registry(&dev.name_id, task.width, task.height, task.x, task.y, task.is_primary);
            }
        }

        // PHASE C: Brute Force Hardware Apply
        unsafe {
            let _ = ChangeDisplaySettingsExW(None, None, None, CDS_TYPE(0), None);
            thread::sleep(Duration::from_millis(250));
            
            let _ = SetDisplayConfig(
                None, 
                None, 
                SDC_APPLY | SDC_ALLOW_CHANGES | SDC_TOPOLOGY_INTERNAL
            );
        }
    }

    pub fn full_scan_discovery(&self) -> Vec<DisplayRow> {
        self.log("INIT", "CCD_SCAN_START");
        unsafe {
            let _ = SetDisplayConfig(None, None, SDC_APPLY | SDC_TOPOLOGY_EXTEND);
            let mut snapshot = Vec::new();
            let fallbacks = [(1920, 1080), (1280, 720)];

            for i in 0..64 {
                let mut device: DISPLAY_DEVICEW = mem::zeroed();
                device.cb = mem::size_of::<DISPLAY_DEVICEW>() as u32;
                if !EnumDisplayDevicesW(None, i, &mut device, 0).as_bool() { break; }

                let name_u16: Vec<u16> = device.DeviceName.iter().take_while(|&&c| c != 0).cloned().collect();
                let pcw_name = PCWSTR(name_u16.as_ptr());
                
                let mut dm: DEVMODEW = mem::zeroed();
                dm.dmSize = mem::size_of::<DEVMODEW>() as u16;

                if !EnumDisplaySettingsW(pcw_name, ENUM_REGISTRY_SETTINGS, &mut dm).as_bool() {
                    let _ = EnumDisplaySettingsW(pcw_name, ENUM_CURRENT_SETTINGS, &mut dm);
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
                    reset_dm.dmFields = DM_PELSWIDTH | DM_PELSHEIGHT;
                    let _ = ChangeDisplaySettingsExW(pcw_name, Some(&reset_dm), None, CDS_UPDATEREGISTRY | CDS_NORESET, None);
                }
            }
            
            self.commit_registry();
            inventory
        }
    }

    fn match_display(&self, row: &DisplayRow, query: &str) -> bool {
        let q = query.to_uppercase();
        row.persistent_id.to_string() == q || 
        row.name_id.to_uppercase() == q ||
        row.position_instance.split('\\').nth(1).map_or(false, |s| s.to_uppercase() == q)
    }

    fn commit_registry(&self) {
        unsafe { let _ = ChangeDisplaySettingsExW(None, None, None, CDS_TYPE(0), None); }
    }

    fn force_set_registry(&self, name: &str, w: i32, h: i32, x: i32, y: i32, primary: bool) {
        let name_u16: Vec<u16> = name.encode_utf16().chain(iter::once(0)).collect();
        let pcw_name = PCWSTR(name_u16.as_ptr());
        unsafe {
            let mut dm: DEVMODEW = mem::zeroed();
            dm.dmSize = mem::size_of::<DEVMODEW>() as u16;
            if EnumDisplaySettingsW(pcw_name, ENUM_REGISTRY_SETTINGS, &mut dm).as_bool() {
                dm.dmPelsWidth = w as u32;
                dm.dmPelsHeight = h as u32;
                dm.dmFields = DM_PELSWIDTH | DM_PELSHEIGHT | DM_POSITION;
                dm.Anonymous1.Anonymous2.dmPosition.x = x;
                dm.Anonymous1.Anonymous2.dmPosition.y = y;
                let mut flags = CDS_UPDATEREGISTRY | CDS_NORESET;
                if primary { flags |= CDS_SET_PRIMARY; }
                let _ = ChangeDisplaySettingsExW(pcw_name, Some(&dm), None, flags, None);
            }
        }
    }
}