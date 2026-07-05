//! Display Controller - Main Orchestrator
//! 
//! Coordinates display operations using modular components

use crate::ddc_control::DdcControl;
use crate::display_apply::apply;
use crate::config::SuiteConfig;
use crate::display_snapshot::{take_snapshot, DisplayRestorer};

pub struct DisplayLogic {
    ddc_control: DdcControl,
}

impl DisplayLogic {
    pub fn new() -> Self {
        Self {
            ddc_control: DdcControl::new(),
        }
    }

    pub fn inventory(&self) -> (Vec<crate::scraper::DisplayRow>, DisplayRestorer) {
        let snapshot = take_snapshot();
        use crate::scraper;
        
        
        // Activate all to ensure edid is parsed.
        let fallbacks = [(1920, 1080), (1280, 720)];
        for (name_u16, is_active, dm) in &snapshot {
            if !*is_active {
                // Stage a dummy resolution in the registry so the scraper can see the device.
                for (w, h) in fallbacks {
                    let mut temp_dm = *dm;
                    temp_dm.dmPelsWidth = w;
                    temp_dm.dmPelsHeight = h;
                    temp_dm.dmFields = windows::Win32::Graphics::Gdi::DM_PELSWIDTH | windows::Win32::Graphics::Gdi::DM_PELSHEIGHT;
                    use crate::display_snapshot::stage_registry_setting;
                    use windows::core::PCWSTR;
                    if stage_registry_setting(PCWSTR(name_u16.as_ptr()), &temp_dm, 0) { break; }
                }
            }
        }
        use crate::display_snapshot::commit_registry;
        commit_registry();
        
        let guard = DisplayRestorer { snapshot };
        (scraper::collect_inventory(), guard)
    }

    pub fn apply(&self, tasks: Vec<crate::scraper::DisplayTask>, _clones: Vec<(String, String)>) {
        apply(&self.ddc_control.tx, tasks);
    }

    pub fn list_suites(&self) -> Vec<String> {
        crate::config::ProfileManager::list_suites()
    }

    pub fn apply_registry_suite(&self, name: &str, silent: bool) -> anyhow::Result<()> {
        let config = SuiteConfig::load(name)?;
        
        if !config.tasks.is_empty() {
            self.apply(config.tasks, vec![]);
            if !silent {
                // Run post-action
                if let Some(cmd) = config.post_cmd {
                    let _ = std::process::Command::new("cmd").args(["/C", &cmd]).spawn();
                }
            }
        }
        Ok(())
    }
}