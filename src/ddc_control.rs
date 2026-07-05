//! DDC/CI Hardware Control
//! 
//! Controls monitor hardware settings (brightness, contrast) via I2C

use std::sync::mpsc::{self, Sender};
use std::thread;
use std::time::Duration;
use crate::scraper::ddc;
use windows::Win32::Graphics::Gdi::HMONITOR;
// (removed unused imports)

/// DDC Command enum
pub enum DdcCommand {
    Apply {
        h_monitor: isize,
        brightness: Option<u32>,
        contrast: Option<u32>,
        delay: bool,
    },
}

/// DDC Control channel
pub struct DdcControl {
    pub tx: Sender<DdcCommand>,
}

impl DdcControl {
    /// Create new DDC control channel
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel::<DdcCommand>();
        // DDC/CI (I2C) operations are notoriously slow and can block.
        // offload to a background thread to prevent UI/CLI lag.
        thread::spawn(move || {
            while let Ok(cmd) = rx.recv() {
                match cmd {
                    DdcCommand::Apply { h_monitor, brightness, contrast, delay } => {
                        if delay {
                            // If a monitor just woke up, the scaler/firmware often 
                            // needs a moment before it starts responding to VCP
                            // Wait for hardware to be ready
                            thread::sleep(Duration::from_millis(800));
                        }
                        let hmon = HMONITOR(h_monitor);
                        ddc::set_monitor_vcp(hmon, brightness, contrast);
                    }
                }
            }
        });
        Self { tx }
    }

    /// Find HMONITOR handle by device name
    pub fn find_hmonitor_by_name(target_name: &str) -> Option<HMONITOR> {
        use windows::Win32::Foundation::LPARAM;
        use windows::Win32::Graphics::Gdi::{GetMonitorInfoW, MONITORINFOEXW};
        
        struct EnumCtx { 
            target: String, 
            result: Option<HMONITOR> 
        }
        let mut ctx = EnumCtx { 
            target: target_name.to_string(), 
            result: None 
        };

        unsafe extern "system" fn callback(h_mon: HMONITOR, _: windows::Win32::Graphics::Gdi::HDC, _: *mut windows::Win32::Foundation::RECT, lparam: LPARAM) -> windows::Win32::Foundation::BOOL {
            let ctx = &mut *(lparam.0 as *mut EnumCtx);
            let mut mi = MONITORINFOEXW::default();
            mi.monitorInfo.cbSize = std::mem::size_of::<MONITORINFOEXW>() as u32;
            if GetMonitorInfoW(h_mon, &mut mi.monitorInfo).as_bool() {
                let device_name = String::from_utf16_lossy(&mi.szDevice).trim_matches(char::from(0)).to_string();
                if device_name == ctx.target {
                    ctx.result = Some(h_mon);
                    return windows::Win32::Foundation::BOOL(0);  // Match found, stop enumeration.
                }
            }
            windows::Win32::Foundation::BOOL(1)
        }
        unsafe { 
            let _ = windows::Win32::Graphics::Gdi::EnumDisplayMonitors(
                None, None, Some(callback), 
                windows::Win32::Foundation::LPARAM(&mut ctx as *mut _ as isize)
            ); 
        }
        ctx.result
    }

    /// Apply DDC settings to monitor
    pub fn apply_ddc(&self, h_monitor: isize, brightness: Option<u32>, contrast: Option<u32>, delay: bool) {
        let _ = self.tx.send(DdcCommand::Apply {
            h_monitor,
            brightness,
            contrast,
            delay,
        });
    }
}