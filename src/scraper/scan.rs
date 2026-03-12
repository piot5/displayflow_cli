use std::mem;
use windows::core::PCWSTR;
use windows::Win32::Foundation::POINT;
use windows::Win32::Graphics::Gdi::{
    EnumDisplayDevicesW, MonitorFromPoint, 
    DISPLAY_DEVICEW, ENUM_CURRENT_SETTINGS, 
    DISPLAY_DEVICE_ATTACHED_TO_DESKTOP, DISPLAY_DEVICE_PRIMARY_DEVICE,
    EnumDisplaySettingsW, DEVMODEW, MONITOR_DEFAULTTONEAREST
};
use windows::Win32::UI::HiDpi::{
    GetDpiForMonitor, MDT_EFFECTIVE_DPI, SetProcessDpiAwarenessContext, 
    DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2
};
use winreg::enums::*;
use winreg::RegKey;
use super::DisplayRow;

pub fn set_dpi_awareness() {
    unsafe { 
        let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2); 
    }
}

pub fn scan_gdi_live() -> Vec<DisplayRow> {
    let mut rows = Vec::new();
    unsafe {
        for i in 0..64 {
            let mut device = DISPLAY_DEVICEW {
                cb: mem::size_of::<DISPLAY_DEVICEW>() as u32,
                ..mem::zeroed()
            };
            
            if !EnumDisplayDevicesW(None, i, &mut device, 0).as_bool() { 
                break; 
            }

            let device_name_raw = PCWSTR::from_raw(device.DeviceName.as_ptr());
            let device_name = String::from_utf16_lossy(&device.DeviceName)
                .trim_matches('\0')
                .to_string();
            
            let mut monitor_device = DISPLAY_DEVICEW {
                cb: mem::size_of::<DISPLAY_DEVICEW>() as u32,
                ..mem::zeroed()
            };
            
            let mut hw_id_path = String::new();
            if EnumDisplayDevicesW(device_name_raw, 0, &mut monitor_device, 0).as_bool() {
                hw_id_path = String::from_utf16_lossy(&monitor_device.DeviceID)
                    .trim_matches('\0')
                    .to_string();
            }

            let is_active = (device.StateFlags & DISPLAY_DEVICE_ATTACHED_TO_DESKTOP) != 0;
            let mut row = DisplayRow {
                source: "GDI_LIVE".into(),
                name_id: device_name.clone(),
                resolution: "N/A".into(),
                freq: "N/A".into(),
                dpi: "100%".into(),
                position_instance: hw_id_path,
                active: if is_active { "YES".into() } else { "NO".into() },
                primary: if (device.StateFlags & DISPLAY_DEVICE_PRIMARY_DEVICE) != 0 { "YES".into() } else { "NO".into() },
                serial: "N/A".into(),
                size_mm: "N/A".into(),
                persistent_id: 0,
            };

            if is_active {
                let mut settings = DEVMODEW {
                    dmSize: mem::size_of::<DEVMODEW>() as u16,
                    ..mem::zeroed()
                };
                
                if EnumDisplaySettingsW(device_name_raw, ENUM_CURRENT_SETTINGS, &mut settings).as_bool() {
                    row.resolution = format!("{}x{}", settings.dmPelsWidth, settings.dmPelsHeight);
                    row.freq = format!("{}", settings.dmDisplayFrequency);
                    
                    // Verschachteltes unsafe entfernt, da wir bereits im unsafe-Block sind
                    let point = POINT { 
                        x: settings.Anonymous1.Anonymous2.dmPosition.x, 
                        y: settings.Anonymous1.Anonymous2.dmPosition.y 
                    };
                    row.dpi = get_dpi_for_point(point);
                }
            }
            rows.push(row);
        }
    }
    rows
}

fn get_dpi_for_point(pt: POINT) -> String {
    unsafe {
        let h_monitor = MonitorFromPoint(pt, MONITOR_DEFAULTTONEAREST);
        let (mut dx, mut dy) = (0, 0);
        if GetDpiForMonitor(h_monitor, MDT_EFFECTIVE_DPI, &mut dx, &mut dy).is_ok() {
            let scale = (dx as f32 / 96.0 * 100.0).round() as i32;
            return format!("{}%", scale);
        }
    }
    "100%".into()
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
                                source: "Registry".into(),
                                name_id: hw_id.clone(),
                                resolution: res,
                                freq: "N/A".into(),
                                dpi: "N/A".into(),
                                position_instance: inst_id.clone(),
                                active: "STORED".into(),
                                primary: "N/A".into(),
                                serial: sn,
                                size_mm: size,
                                persistent_id: 0,
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
    if bytes.len() < 128 { 
        return ("N/A".into(), "N/A".into(), "N/A".into()); 
    }
    
    let mut sn = "N/A".to_string();
    for offset in [54, 72, 90, 108] {
        if offset + 18 <= bytes.len() && &bytes[offset..offset+4] == b"\x00\x00\x00\xff" {
            sn = String::from_utf8_lossy(&bytes[offset+4..offset+18])
                .trim()
                .trim_matches('\0')
                .to_string();
            break;
        }
    }
    
    let w_mm = (bytes[21] as u32) * 10;
    let h_mm = (bytes[22] as u32) * 10;
    let w_px = ((bytes[58] as u32 & 0xF0) << 4) | (bytes[56] as u32);
    let h_px = ((bytes[61] as u32 & 0xF0) << 4) | (bytes[59] as u32);
    
    (sn, format!("{}x{}mm", w_mm, h_mm), format!("{}x{}", w_px, h_px))
}