use windows::Win32::Graphics::Gdi::*;
use windows::Win32::Devices::Display::*;
use windows::Win32::Foundation::{RECT, BOOL, LPARAM};

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone, Default)]
pub struct DdcCaps {
    pub brightness: u32,
    pub contrast: u32,
    pub input_source: u32,
}

pub fn get_monitor_caps(h_monitor: HMONITOR) -> Option<DdcCaps> {
    unsafe {
        let mut count = 0;
        if GetNumberOfPhysicalMonitorsFromHMONITOR(h_monitor, &mut count).is_err() { return None; }
        let mut phys_monitors = vec![PHYSICAL_MONITOR::default(); count as usize];
        if GetPhysicalMonitorsFromHMONITOR(h_monitor, &mut phys_monitors).is_err() { return None; }

        let mut caps = DdcCaps::default();
        if let Some(mon) = phys_monitors.first() {
            let mut min = 0; let mut cur = 0; let mut max = 0;
            
            // Fix: Direkter Vergleich mit 0 statt .as_bool() für i32 Rückgaben
            if GetMonitorBrightness(mon.hPhysicalMonitor, &mut min, &mut cur, &mut max) != 0 {
                caps.brightness = cur;
            }
            if GetMonitorContrast(mon.hPhysicalMonitor, &mut min, &mut cur, &mut max) != 0 {
                caps.contrast = cur;
            }

            let mut vcp_type = MC_VCP_CODE_TYPE(0);
            let mut vcp_cur = 0;
            let mut vcp_max = 0;
            // Fix: GetVCPFeatureAndVCPFeatureReply liefert ebenfalls i32
            if GetVCPFeatureAndVCPFeatureReply(mon.hPhysicalMonitor, 0x60, Some(&mut vcp_type), &mut vcp_cur, Some(&mut vcp_max)) != 0 {
                caps.input_source = vcp_cur;
            }
            
            let _ = DestroyPhysicalMonitors(&phys_monitors);
        }
        Some(caps)
    }
}

pub extern "system" fn monitor_enum_proc(h_monitor: HMONITOR, _: HDC, _: *mut RECT, lparam: LPARAM) -> BOOL {
    let monitors = unsafe { &mut *(lparam.0 as *mut Vec<(HMONITOR, DdcCaps)>) };
    if let Some(caps) = get_monitor_caps(h_monitor) {
        monitors.push((h_monitor, caps));
    }
    BOOL(1)
}

pub fn set_monitor_vcp(h_monitor: HMONITOR, brightness: Option<u32>, contrast: Option<u32>) {
    unsafe {
        let mut count = 0;
        if GetNumberOfPhysicalMonitorsFromHMONITOR(h_monitor, &mut count).is_err() { return; }
        let mut phys_monitors = vec![PHYSICAL_MONITOR::default(); count as usize];
        if GetPhysicalMonitorsFromHMONITOR(h_monitor, &mut phys_monitors).is_err() { return; }

        if let Some(mon) = phys_monitors.first() {
            if let Some(b) = brightness {
                let _ = SetMonitorBrightness(mon.hPhysicalMonitor, b);
            }
            if let Some(c) = contrast {
                let _ = SetMonitorContrast(mon.hPhysicalMonitor, c);
            }
            let _ = DestroyPhysicalMonitors(&phys_monitors);
        }
    }
}