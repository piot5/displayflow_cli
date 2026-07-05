//! Config-based Suite Storage (replaces Registry storage)
//! 
//! Suites are stored as JSON files in %LOCALAPPDATA%\DisplayFlow\suites\

use std::env;
use std::fs;
use std::path::PathBuf;
use anyhow::{Result, Context};
use serde::{Serialize, Deserialize};
use crate::scraper::DisplayTask;

/// Config file path for suites
fn get_suites_dir() -> PathBuf {
    let local_appdata = env::var("LOCALAPPDATA")
        .unwrap_or_else(|_| ".\\suites".to_string());
    PathBuf::from(local_appdata)
        .join("DisplayFlow")
        .join("suites")
}

/// Suite configuration stored in JSON
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SuiteConfig {
    pub name: String,
    #[serde(default)]
    pub tasks: Vec<DisplayTask>,
    pub hotkey: Option<String>,
    pub post_cmd: Option<String>,
}

impl Default for SuiteConfig {
    fn default() -> Self {
        Self {
            name: String::new(),
            tasks: Vec::new(),
            hotkey: None,
            post_cmd: None,
        }
    }
}

impl SuiteConfig {
    /// Load suite config from JSON file
    pub fn load(name: &str) -> Result<Self> {
        let config_path = get_suites_dir().join(format!("{}.json", name));
        let content = fs::read_to_string(&config_path)
            .with_context(|| format!("Failed to read suite '{}' from {:?}", name, config_path))?;
        let config: SuiteConfig = serde_json::from_str(&content)
            .with_context(|| format!("Failed to parse suite '{}' JSON", name))?;
        Ok(config)
    }
}

/// Profile Manager - now using JSON config files instead of Registry
pub struct ProfileManager;

impl ProfileManager {
    /// Create a suite configuration file
    pub fn create_suite(
        name: &str, 
        tasks: &[DisplayTask], 
        hotkey: Option<String>, 
        post_cmd: Option<String>,
        _create_link: bool,
        _link_with_hotkey: bool
    ) -> Result<()> {
        let base_name = name.strip_suffix(".bat").unwrap_or(name);
        
        // Ensure suites directory exists
        let suites_dir = get_suites_dir();
        fs::create_dir_all(&suites_dir)
            .context("Failed to create suites directory")?;
        
        // Create suite config
        let config = SuiteConfig {
            name: base_name.to_string(),
            tasks: tasks.to_vec(),
            hotkey,
            post_cmd,
        };
        
        // Write to JSON file
        let config_path = suites_dir.join(format!("{}.json", base_name));
        let json_content = serde_json::to_string_pretty(&config)
            .context("Failed to serialize suite config")?;
        fs::write(&config_path, json_content)
            .context("Failed to write suite config file")?;
        
        log::info!("Suite '{}' saved to {:?}", base_name, config_path);
        Ok(())
    }

    /// List all saved suite names
    pub fn list_suites() -> Vec<String> {
        let suites_dir = get_suites_dir();
        match fs::read_dir(&suites_dir) {
            Ok(entries) => entries
                .filter_map(|e| e.ok())
                .filter_map(|e| {
                    let name = e.file_name().to_string_lossy().into_owned();
                    if name.ends_with(".json") {
                        Some(name.trim_end_matches(".json").to_string())
                    } else {
                        None
                    }
                })
                .collect(),
            Err(_) => Vec::new(),
        }
    }

// load_suite entfernt - verwende stattdessen SuiteConfig::load

    /// Capture hotkey (moved from profiles.rs - unchanged logic)
    pub fn capture_hotkey() -> Option<String> {
        use std::thread;
        use std::time::Duration;
        use windows::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_CONTROL, VK_MENU, VK_SHIFT};
        
        println!("Press your Hotkey combination (e.g., Ctrl+Alt+D)...");
        loop {
            unsafe {
                let mut combo = Vec::new();
                if (GetAsyncKeyState(VK_CONTROL.0 as i32) as u16 & 0x8000) != 0 { combo.push("Ctrl".into()); }
                if (GetAsyncKeyState(VK_MENU.0 as i32) as u16 & 0x8000) != 0 { combo.push("Alt".into()); }
                if (GetAsyncKeyState(VK_SHIFT.0 as i32) as u16 & 0x8000) != 0 { combo.push("Shift".into()); }

                for k in 0x41..0x5B {
                    if (GetAsyncKeyState(k as i32) as u16 & 0x8000) != 0 {
                        combo.push((k as u8 as char).to_string());
                        let res = combo.join("+");
                        while Self::is_any_modifier_down() { 
                            thread::sleep(Duration::from_millis(10)); 
                        }
                        return Some(res);
                    }
                }
            }
            thread::sleep(Duration::from_millis(50));
        }
    }

    fn is_any_modifier_down() -> bool {
        use windows::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_CONTROL, VK_MENU, VK_SHIFT};
        unsafe {
            (GetAsyncKeyState(VK_CONTROL.0 as i32) as u16 & 0x8000 != 0) ||
            (GetAsyncKeyState(VK_MENU.0 as i32) as u16 & 0x8000 != 0) ||
            (GetAsyncKeyState(VK_SHIFT.0 as i32) as u16 & 0x8000 != 0)
        }
    }
}