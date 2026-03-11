use std::mem;
use windows::core::PCWSTR;
use windows::Win32::Foundation::{BOOL, LPARAM, RECT};
use windows::Win32::Graphics::Gdi::{
    EnumDisplayDevicesW, EnumDisplayMonitors, EnumDisplaySettingsW, GetMonitorInfoW,
    DISPLAY_DEVICEW, ENUM_CURRENT_SETTINGS, HDC, HMONITOR, MONITORINFOEXW, DEVMODEW, 
    DISPLAY_DEVICE_ATTACHED_TO_DESKTOP, DISPLAY_DEVICE_PRIMARY_DEVICE
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
            let mut device: DISPLAY_DEVICEW = mem::zeroed();
            device.cb = mem::size_of::<DISPLAY_DEVICEW>() as u32;
            
            if !EnumDisplayDevicesW(None, i, &mut device, 0).as_bool() { 
                break; 
            }

            let device_name = String::from_utf16_lossy(&device.DeviceName)
                .trim_matches('\0')
                .to_string();
            
            let mut monitor_device: DISPLAY_DEVICEW = mem::zeroed();
            monitor_device.cb = mem::size_of::<DISPLAY_DEVICEW>() as u32;
            let mut hw_id_path = String::new();
            
            if EnumDisplayDevicesW(
                PCWSTR::from_raw(device.DeviceName.as_ptr()), 
                0, 
                &mut monitor_device, 
                0
            ).as_bool() {
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
                dpi: get_dpi_scaling_from_name(&device_name),
                position_instance: hw_id_path,
                active: if is_active { "YES".into() } else { "NO".into() },
                primary: if (device.StateFlags & DISPLAY_DEVICE_PRIMARY_DEVICE) != 0 { "YES".into() } else { "NO".into() },
                serial: "N/A".into(),
                size_mm: "N/A".into(),
                persistent_id: 0,
            };

            if is_active {
                let mut settings: DEVMODEW = mem::zeroed();
                settings.dmSize = mem::size_of::<DEVMODEW>() as u16;
                if EnumDisplaySettingsW(
                    PCWSTR::from_raw(device.DeviceName.as_ptr()), 
                    ENUM_CURRENT_SETTINGS, 
                    &mut settings
                ).as_bool() {
                    row.resolution = format!("{}x{}", settings.dmPelsWidth, settings.dmPelsHeight);
                    row.freq = format!("{}", settings.dmDisplayFrequency);
                }
            }
            rows.push(row);
        }
    }
    rows
}

pub fn scan_registry_monitors() -> Vec<DisplayRow> {
    let mut results = Vec::new();
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    
    if let Ok(display_key) = hklm.open_subkey(r"SYSTEM\CurrentControlSet\Enum\DISPLAY") {
        for hw_id in display_key.enum_keys().map_while(Result::ok) {
            if let Ok(hw_key) = display_key.open_subkey(&hw_id) {
                for inst_id in hw_key.enum_keys().map_while(Result::ok) {
                    let path = format!(r"{}\Device Parameters", inst_id);
                    if let Ok(params) = hw_key.open_subkey(path) {
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
        }
    }
    
    let (w_mm, h_mm) = ((bytes[21] as u32) * 10, (bytes[22] as u32) * 10);
    let w_px = ((bytes[58] as u32 & 0xF0) << 4) | (bytes[56] as u32);
    let h_px = ((bytes[61] as u32 & 0xF0) << 4) | (bytes[59] as u32);
    
    (sn, format!("{}x{}mm", w_mm, h_mm), format!("{}x{}", w_px, h_px))
}

fn get_dpi_scaling_from_name(_name: &str) -> String {
    let mut dpi_str = "100%".to_string();
    unsafe { 
        let _ = EnumDisplayMonitors(
            None, 
            None, 
            Some(monitor_enum_proc), 
            LPARAM(&mut dpi_str as *mut String as isize)
        ); 
    }
    dpi_str
}

unsafe extern "system" fn monitor_enum_proc(h: HMONITOR, _: HDC, _: *mut RECT, p: LPARAM) -> BOOL {
    let mut mi: MONITORINFOEXW = mem::zeroed();
    mi.monitorInfo.cbSize = mem::size_of::<MONITORINFOEXW>() as u32;
    
    if GetMonitorInfoW(h, &mut mi.monitorInfo).as_bool() {
        let (mut dx, mut dy) = (0, 0);
        if GetDpiForMonitor(h, MDT_EFFECTIVE_DPI, &mut dx, &mut dy).is_ok() {
            let scale = (dx as f32 / 96.0 * 100.0) as i32;
            *(p.0 as *mut String) = format!("{}%", scale);
        }
    }
    BOOL(1)
}