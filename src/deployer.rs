use std::{fs, io, process::Command, env, thread, time::Duration};
use crate::scraper::DisplayTask;
use windows::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_CONTROL, VK_MENU, VK_SHIFT};

pub struct DeploymentManager;

impl DeploymentManager {
    pub fn create_suite(
        name: &str, 
        tasks: &[DisplayTask], 
        hotkey_string: Option<String>, 
        post_cmd: Option<String>
    ) -> Result<(), io::Error> {
        let current_dir = env::current_dir()?;
        let base_name = name.strip_suffix(".bat").unwrap_or(name);

        let bat_path = current_dir.join(format!("{}.bat", base_name));
        let vbs_path = current_dir.join(format!("{}.vbs", base_name));
        let icon_path = current_dir.join("DF.ico");

        
        let args = tasks.iter().map(|t| {
            let rot = t.direction.as_deref().unwrap_or("0");
            format!("\"{}:{}:{}:{}:{}:{}:{}\"", 
                t.query, t.width, t.height, t.x, t.y, 
                if t.is_primary { 1 } else { 0 }, rot)
        }).collect::<Vec<_>>().join(" ");

        
        let mut bat_content = String::from("@echo off\n\n");
        
        
        bat_content.push_str("start \"\" \"%~dp0screen_animation.exe\" -d left right\n");
        bat_content.push_str("powershell -command \"Start-Sleep -Milliseconds 3200\"\n\n");
        
        
        bat_content.push_str(&format!("\"%~dp0displayflow.exe\" {}\n", args));
        
        if let Some(cmd) = post_cmd { 
            bat_content.push_str(&format!("\nstart \"\" \"{}\"", cmd)); 
        }
        
        fs::write(&bat_path, bat_content)?;

        // VBS-Wrapper für versteckte Ausführung (verweist auf die .bat)
        let vbs_content = format!(
            "Set W = CreateObject(\"WScript.Shell\")\nW.Run \"cmd /c \"\"{}\"\"\", 0", 
            bat_path.display()
        );
        fs::write(&vbs_path, vbs_content)?;

        
        if let Some(hk_string) = hotkey_string {
            Self::create_desktop_shortcut(
                base_name, 
                &vbs_path.to_string_lossy(), 
                &icon_path.to_string_lossy(), 
                &hk_string
            );
        }

        Ok(())
    }

    pub fn capture_hotkey_physical() -> Option<String> {
        thread::sleep(Duration::from_millis(500));

        let result = loop {
            let mut combo: Vec<String> = Vec::new();
            let mut key_pressed = false;

            unsafe {
                let ctrl = GetAsyncKeyState(VK_CONTROL.0 as i32) as u16 & 0x8000 != 0;
                let alt = GetAsyncKeyState(VK_MENU.0 as i32) as u16 & 0x8000 != 0;
                let shift = GetAsyncKeyState(VK_SHIFT.0 as i32) as u16 & 0x8000 != 0;

                if ctrl { combo.push("CTRL".into()); }
                if alt { combo.push("ALT".into()); }
                if shift { combo.push("SHIFT".into()); }
                
                for vk in (0x41..=0x5A).chain(0x70..=0x7B) {
                    if GetAsyncKeyState(vk as i32) as u16 & 0x8000 != 0 {
                        let name = match vk {
                            0x70..=0x7B => format!("F{}", vk - 0x6F),
                            _ => (vk as u8 as char).to_string(),
                        };
                        combo.push(name);
                        key_pressed = true;
                        break;
                    }
                }

                if combo.len() >= 2 && key_pressed {
                    let final_str = combo.join("+");
                    
                    while (GetAsyncKeyState(VK_CONTROL.0 as i32) as u16 & 0x8000 != 0) ||
                          (GetAsyncKeyState(VK_MENU.0 as i32) as u16 & 0x8000 != 0) ||
                          (GetAsyncKeyState(VK_SHIFT.0 as i32) as u16 & 0x8000 != 0) {
                        thread::sleep(Duration::from_millis(10));
                    }
                    
                    break Some(final_str);
                }
            }
            thread::sleep(Duration::from_millis(50));
        };
        
        result
    }

    fn create_desktop_shortcut(name: &str, vbs_path: &str, icon_path: &str, hk: &str) {
        let ps_script = format!(
            "$ws = New-Object -ComObject WScript.Shell; \
             $desktop = [Environment]::GetFolderPath('Desktop'); \
             $lnkPath = Join-Path $desktop 'df_{}.lnk'; \
             $s = $ws.CreateShortcut($lnkPath); \
             $s.TargetPath = 'wscript.exe'; \
             $s.Arguments = '\"\"\"{}\"\"\"'; \
             $s.Hotkey = '{}'; \
             if (Test-Path '{}') {{ $s.IconLocation = '{}' }}; \
             $s.Save();",
            name, vbs_path, hk, icon_path, icon_path
        );

        let _ = Command::new("powershell")
            .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", &ps_script])
            .output();
    }
}