use std::{process::Command, env, thread, time::Duration};
use crate::scraper::DisplayTask;
use windows::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_CONTROL, VK_MENU, VK_SHIFT};
use winreg::enums::*;
use winreg::RegKey;
use anyhow::{Result, Context, anyhow};

const SUITE_REG_PATH: &str = r"Software\DisplayFlow\Suites";

pub struct DeploymentManager;
impl DeploymentManager {
    pub fn create_suite(
        name: &str, 
        tasks: &[DisplayTask], 
        hotkey: Option<String>, 
        post_cmd: Option<String>, 
        create_link: bool,
        link_with_hotkey: bool
    ) -> Result<()> {
        let base_name = name.strip_suffix(".bat").unwrap_or(name);
        let task_data = Self::format_task_args(tasks);
        
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let (key, _) = hkcu.create_subkey(SUITE_REG_PATH)
            .context("Failed to access registry")?;

        let mut suite_config = format!("tasks|{}", task_data);
        if let Some(cmd) = post_cmd { suite_config.push_str(&format!(";post|{}", cmd)); }
        
        if let Some(ref hk) = hotkey {
            suite_config.push_str(&format!(";hotkey|{}", hk));
        }

        key.set_value(base_name, &suite_config)?;

        if create_link {
            let exe_path = env::current_exe()?
                .to_str()
                .ok_or_else(|| anyhow!("Path error"))?
                .to_string();
            let icon_path = env::current_dir()?
                .join("DF.ico")
                .to_str()
                .unwrap_or("")
                .to_string();
            
            let shell_hk = if link_with_hotkey { hotkey.unwrap_or_default() } else { String::new() };

            Self::create_desktop_shortcut(base_name, &exe_path, &format!("--apply-suite \"{}\"", base_name), &icon_path, &shell_hk)?;
        }

        Ok(())
    }

    fn format_task_args(tasks: &[DisplayTask]) -> String {
        tasks.iter().map(|t| {
            let ani = t.animation.as_deref().unwrap_or("0");
            let dir = t.direction.as_deref().unwrap_or("0");
            format!(
                "{}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}", 
                t.query, 
                t.width, 
                t.height, 
                t.x, 
                t.y, 
                if t.is_primary { "1" } else { "0" },
                dir,
                t.freq,
                t.brightness.unwrap_or(999),
                t.contrast.unwrap_or(999),
                ani
            )
        }).collect::<Vec<_>>().join(",")
    }

    pub fn capture_hotkey() -> Option<String> {
        let mut combo = Vec::new();
        loop {
            unsafe {
                if (GetAsyncKeyState(VK_CONTROL.0 as i32) as u16 & 0x8000) != 0 { if !combo.contains(&"Ctrl".into()) { combo.push("Ctrl".into()); } }
                if (GetAsyncKeyState(VK_MENU.0 as i32) as u16 & 0x8000) != 0 { if !combo.contains(&"Alt".into()) { combo.push("Alt".into()); } }
                if (GetAsyncKeyState(VK_SHIFT.0 as i32) as u16 & 0x8000) != 0 { if !combo.contains(&"Shift".into()) { combo.push("Shift".into()); } }

                for k in 0x41..0x5B {
                    if (GetAsyncKeyState(k) as u16 & 0x8000) != 0 {
                        combo.push((k as u8 as char).to_string());
                        let res = combo.join("+");
                        while Self::is_any_modifier_down() { thread::sleep(Duration::from_millis(10)); }
                        return Some(res);
                    }
                }
            }
            thread::sleep(Duration::from_millis(50));
        }
    }

    fn is_any_modifier_down() -> bool {
        unsafe {
            (GetAsyncKeyState(VK_CONTROL.0 as i32) as u16 & 0x8000 != 0) ||
            (GetAsyncKeyState(VK_MENU.0 as i32) as u16 & 0x8000 != 0) ||
            (GetAsyncKeyState(VK_SHIFT.0 as i32) as u16 & 0x8000 != 0)
        }
    }

    fn create_desktop_shortcut(name: &str, exe: &str, args: &str, icon: &str, hk: &str) -> Result<()> {
        let script = format!(
            "$ws = New-Object -ComObject WScript.Shell; \
             $lnk = $ws.CreateShortcut((Join-Path ([Environment]::GetFolderPath('Desktop')) 'df_{}.lnk')); \
             $lnk.TargetPath = '{}'; $lnk.Arguments = '{}'; $lnk.Hotkey = '{}'; \
             if (Test-Path '{}') {{ $lnk.IconLocation = '{}' }}; $lnk.Save();",
            name, exe, args, hk, icon, icon
        );
        let _ = Command::new("powershell").args(["-Command", &script]).status();
        Ok(())
    }
}