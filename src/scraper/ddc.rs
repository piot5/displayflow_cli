use df_ddc::{list_monitors, DisplayDevice};
use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct DdcCaps {
    pub brightness: u32,
    pub contrast: u32,
    pub input_source: u32,
}

/// List DDC-capable monitors and their capabilities using the df_ddc crate.
/// Returns a vector of (device_info, DdcCaps).
pub fn list_ddc_caps() -> Vec<(String, DdcCaps)> {
    let mut out = Vec::new();
    for dev in list_monitors() {
        if let Ok(caps) = dev.inner.get_capabilities() {
            out.push((
                dev.info.clone(),
                DdcCaps {
                    brightness: caps.brightness,
                    contrast: caps.contrast,
                    input_source: caps.input_source,
                },
            ));
        }
    }
    out
}

/// Set monitor brightness/contrast by matching a device info string.
/// This is a best-effort helper for the CLI; it will try to find the first
/// matching device and apply the settings via the df_ddc trait.
pub fn set_monitor_vcp_by_info(target_info: &str, brightness: Option<u32>, contrast: Option<u32>) {
    for dev in list_monitors() {
        if dev.info.contains(target_info) || target_info.contains(&dev.info) {
            if let Some(b) = brightness {
                let _ = dev.inner.set_brightness(b);
            }
            if let Some(c) = contrast {
                let _ = dev.inner.set_vcp_feature(0x12, c);
            }
            break;
        }
    }
}
