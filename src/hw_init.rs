use std::iter;
use windows::core::PCWSTR;
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::Devices::Display::{SetDisplayConfig, SDC_APPLY, SDC_TOPOLOGY_EXTEND};
use crate::scraper::{self, DisplayRow, scan::GdiDevMode};

struct DisplayGuard { snapshot: Vec<(String, bool, DEVMODEW)> }

impl Drop for DisplayGuard {
    fn drop(&mut self) {
        for (name_id, was_active, old_dm) in &self.snapshot {
            let name_u16: Vec<u16> = name_id.encode_utf16().chain(iter::once(0)).collect();
            let pcw_name = PCWSTR(name_u16.as_ptr());
            if !*was_active {
                let mut detach_dm = GdiDevMode::new();
                detach_dm.0.dmPelsWidth = 0;
                detach_dm.0.dmPelsHeight = 0;
                detach_dm.0.dmFields = DM_PELSWIDTH | DM_PELSHEIGHT | DM_POSITION;
                unsafe { let _ = ChangeDisplaySettingsExW(pcw_name, Some(&detach_dm.0), None, CDS_UPDATEREGISTRY | CDS_NORESET, None); }
            } else {
                let mut reset_dm = *old_dm;
                reset_dm.dmFields = DM_PELSWIDTH | DM_PELSHEIGHT | DM_POSITION | DM_DISPLAYORIENTATION;
                unsafe { let _ = ChangeDisplaySettingsExW(pcw_name, Some(&reset_dm), None, CDS_UPDATEREGISTRY | CDS_NORESET, None); }
            }
        }
        unsafe { let _ = ChangeDisplaySettingsExW(None, None, None, CDS_TYPE(0), None); }
    }
}

pub fn run_discovery_flow() -> Vec<DisplayRow> {
    let inventory_initial = scraper::collect_inventory();
    let mut snapshot_data = Vec::new();

    for row in &inventory_initial {
        let mut dm = GdiDevMode::new();
        let name_u16: Vec<u16> = row.name_id.encode_utf16().chain(iter::once(0)).collect();
        let pcw_name = PCWSTR(name_u16.as_ptr());
        unsafe {
            if !EnumDisplaySettingsW(pcw_name, ENUM_REGISTRY_SETTINGS, &mut dm.0).as_bool() {
                let _ = EnumDisplaySettingsW(pcw_name, ENUM_CURRENT_SETTINGS, &mut dm.0);
            }
        }
        let is_active = row.active.to_uppercase() == "YES";
        snapshot_data.push((row.name_id.clone(), is_active, dm.0));
    }

    {
        let _guard = DisplayGuard { snapshot: snapshot_data };
        unsafe { let _ = SetDisplayConfig(None, None, SDC_APPLY | SDC_TOPOLOGY_EXTEND); }

        for row in &inventory_initial {
            if row.active.to_uppercase() != "YES" {
                let name_u16: Vec<u16> = row.name_id.encode_utf16().chain(iter::once(0)).collect();
                let mut temp_dm = GdiDevMode::new();
                temp_dm.0.dmPelsWidth = 1920;
                temp_dm.0.dmPelsHeight = 1080;
                temp_dm.0.dmFields = DM_PELSWIDTH | DM_PELSHEIGHT;
                unsafe { let _ = ChangeDisplaySettingsExW(PCWSTR(name_u16.as_ptr()), Some(&temp_dm.0), None, CDS_UPDATEREGISTRY | CDS_NORESET, None); }
            }
        }
        unsafe { let _ = ChangeDisplaySettingsExW(None, None, None, CDS_TYPE(0), None); }
        scraper::collect_inventory()
    }
}