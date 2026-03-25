use std::mem;
use anyhow::Result;
use windows::core::PCWSTR;
use windows::Win32::Foundation::POINT;
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::UI::HiDpi::*;
use winreg::enums::*;
use winreg::RegKey;
use super::DisplayRow;

pub struct GdiDevice(pub DISPLAY_DEVICEW);
impl GdiDevice {
    pub fn new() -> Self {
        let mut device = DISPLAY_DEVICEW::default();
        device.cb = mem::size_of::<DISPLAY_DEVICEW>() as u32;
        Self(device)
    }
    pub fn as_mut_ptr(&mut self) -> *mut DISPLAY_DEVICEW { &mut self.0 }
}

pub struct GdiDevMode(pub DEVMODEW);
impl GdiDevMode {
    pub fn new() -> Self {
        let mut dm = DEVMODEW::default();
        dm.dmSize = mem::size_of::<DEVMODEW>() as u16;
        Self(dm)
    }
}

pub fn set_dpi_awareness() { 
    unsafe { let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2); } 
}

pub fn scan_gdi_live() -> Result<Vec<DisplayRow>> {
    let mut rows = Vec::new();
    unsafe {
        for i in 0..64 {
            let mut device = GdiDevice::new();
            if !EnumDisplayDevicesW(None, i, device.as_mut_ptr(), 0).as_bool() { break; }
            
            let device_name_raw = PCWSTR::from_raw(device.0.DeviceName.as_ptr());
            let device_name = String::from_utf16_lossy(&device.0.DeviceName).trim_matches('\0').to_string();
            
            let mut monitor_device = GdiDevice::new();
            let mut hw_id_path = String::new();
            if EnumDisplayDevicesW(device_name_raw, 0, monitor_device.as_mut_ptr(), 0).as_bool() {
                hw_id_path = String::from_utf16_lossy(&monitor_device.0.DeviceID).trim_matches('\0').to_string();
            }

            let is_active = (device.0.StateFlags & DISPLAY_DEVICE_ATTACHED_TO_DESKTOP) != 0;
            let is_pri = (device.0.StateFlags & DISPLAY_DEVICE_PRIMARY_DEVICE) != 0;

            let mut row = DisplayRow {
                source: "GDI_LIVE".into(),
                name_id: device_name,
                resolution: "N/A".into(),
                freq: "N/A".into(),
                dpi: "100%".into(),
                scale_factor: 1.0,
                position_instance: hw_id_path,
                active: if is_active { "YES".into() } else { "NO".into() },
                primary: if is_pri { "YES".into() } else { "NO".into() },
                serial: "N/A".into(),
                size_mm: "N/A".into(),
                persistent_id: 0,
                x: 0, y: 0, 
                is_primary: is_pri,
                ddc: None, // Fix für E0063
            };

            if is_active {
                let mut settings = GdiDevMode::new();
                if EnumDisplaySettingsW(device_name_raw, ENUM_CURRENT_SETTINGS, &mut settings.0).as_bool() {
                    row.resolution = format!("{}x{}", settings.0.dmPelsWidth, settings.0.dmPelsHeight);
                    row.freq = format!("{}Hz", settings.0.dmDisplayFrequency);
                    row.x = settings.0.Anonymous1.Anonymous2.dmPosition.x;
                    row.y = settings.0.Anonymous1.Anonymous2.dmPosition.y;
                    
                    let h_monitor = MonitorFromPoint(POINT { x: row.x, y: row.y }, MONITOR_DEFAULTTONEAREST);
                    let (mut dx, mut dy) = (0, 0);
                    if GetDpiForMonitor(h_monitor, MDT_EFFECTIVE_DPI, &mut dx, &mut dy).is_ok() {
                        row.scale_factor = dx as f32 / 96.0;
                        row.dpi = format!("{}%", (row.scale_factor * 100.0) as i32);
                    }
                }
            }
            rows.push(row);
        }
    }
    Ok(rows)
}

pub fn scan_registry_monitors() -> Vec<DisplayRow> {
    let mut results = Vec::new();
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    if let Ok(display_key) = hklm.open_subkey(r"SYSTEM\CurrentControlSet\Enum\DISPLAY") {
        for hw_id in display_key.enum_keys().map_while(Result::ok) {
            if let Ok(hw_key) = display_key.open_subkey(&hw_id) {
                for inst_id in hw_key.enum_keys().map_while(Result::ok) {
                    if let Ok(params) = hw_key.open_subkey(format!(r"{}\Device Parameters", inst_id)) {
                        if let Ok(edid) = params.get_raw_value("EDID") {
                            let (sn, size, res) = parse_edid(&edid.bytes);
                            results.push(DisplayRow {
                                source: "Registry".into(), name_id: hw_id.clone(), resolution: res,
                                freq: "N/A".into(), dpi: "N/A".into(), scale_factor: 1.0, position_instance: inst_id.clone(),
                                active: "STORED".into(), primary: "N/A".into(), serial: sn, size_mm: size,
                                persistent_id: 0, x: 0, y: 0, is_primary: false,
                                ddc: None, // Fix für E0063
                            });
                        }
                    }
                }
            }
        }
    }
    results
}

fn parse_edid(bytes: &[u8]) -> (String, String, String) {
    if bytes.len() < 128 { return ("N/A".into(), "N/A".into(), "N/A".into()); }
    let mut sn = "N/A".to_string();
    for offset in [54, 72, 90, 108] {
        if offset + 18 <= bytes.len() && &bytes[offset..offset+4] == b"\x00\x00\x00\xff" {
            sn = String::from_utf8_lossy(&bytes[offset+4..offset+18]).trim().trim_matches('\0').to_string();
            break;
        }
    }
    let w_mm = (bytes[21] as u32) * 10;
    let h_mm = (bytes[22] as u32) * 10;
    (sn, format!("{}x{}mm", w_mm, h_mm), "N/A".into())
}