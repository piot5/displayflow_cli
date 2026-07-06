use df_ddc::{list_monitors, DdcControl as DdcControlTrait};
use std::sync::mpsc::{self, Sender};
use std::thread;
use std::time::Duration;

/// DDC Command enum
pub enum DdcCommand {
    ApplyByIndex { idx: usize, brightness: Option<u32>, contrast: Option<u32>, delay: bool },
}

/// DDC Control channel (wrapper around df_ddc backends).
/// This spawns a background thread which owns the concrete backend objects
/// and performs potentially blocking operations off the main thread.
pub struct DdcControl {
    pub tx: Sender<DdcCommand>,
    /// Human-readable device infos (index-aligned with the backends in the thread)
    pub devices: Vec<String>,
}

impl DdcControl {
    /// Create new DDC control channel and spawn worker thread.
    pub fn new() -> Self {
        let devices_list = list_monitors();
        let device_infos: Vec<String> = devices_list.iter().map(|d| d.info.clone()).collect();

        // Move the boxed backends into the worker thread
        let mut backends: Vec<Box<dyn DdcControlTrait>> = devices_list.into_iter().map(|d| d.inner).collect();

        let (tx, rx) = mpsc::channel::<DdcCommand>();
        thread::spawn(move || {
            while let Ok(cmd) = rx.recv() {
                match cmd {
                    DdcCommand::ApplyByIndex { idx, brightness, contrast, delay } => {
                        if delay {
                            thread::sleep(Duration::from_millis(800));
                        }
                        if let Some(backend) = backends.get_mut(idx) {
                            if let Some(b) = brightness {
                                let _ = backend.set_brightness(b);
                            }
                            if let Some(c) = contrast {
                                let _ = backend.set_vcp_feature(0x12, c);
                            }
                        }
                    }
                }
            }
        });

        Self { tx, devices: device_infos }
    }

    /// Try to find a device index by matching the device info string.
    pub fn find_device_index_by_name(&self, target_name: &str) -> Option<usize> {
        self.devices.iter().position(|i| i.contains(target_name) || target_name.contains(i))
    }

    /// Apply DDC settings by device index (non-blocking, performed by worker thread)
    pub fn apply_by_index(&self, idx: usize, brightness: Option<u32>, contrast: Option<u32>, delay: bool) {
        let _ = self.tx.send(DdcCommand::ApplyByIndex { idx, brightness, contrast, delay });
    }
}
