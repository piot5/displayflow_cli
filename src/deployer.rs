use std::{fs, io, process::Command, env, thread, time::Duration};
use crate::scraper::DisplayTask;
use windows::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_CONTROL, VK_MENU, VK_SHIFT};

pub struct DeploymentManager;

impl DeploymentManager {
    
    pub fn create_suite(
        name: &str, 
        tasks: &[DisplayTask], 
        hotkey: Option<String>, 
        post_cmd: Option<String>
    ) -> Result<(), io::Error> {
        let base_name = name.strip_suffix(".bat").unwrap_or(name);
        let root = env::current_dir()?;

        let paths = (
            root.join(format!("{}.bat", base_name)),
            root.join(format!("{}.vbs", base_name)),
            root.join("DF.ico")
        );

        Self::write_batch_payload(&paths.0, tasks, post_cmd)?;
        Self::write_vbs_wrapper(&paths.1, &paths.0)?;

        if let Some(hk) = hotkey {
            Self::create_desktop_shortcut(
                base_name, 
                &paths.1.to_string_lossy(), 
                &paths.2.to_string_lossy(), 
                &hk
            )?;
        }

        Ok(())
    }

    
    fn format_task_args(tasks: &[DisplayTask]) -> String {
        tasks.iter().map(|t| {
            let rot = t.direction.as_deref().unwrap_or("0");
            format!(
                "\"{}:{}:{}:{}:{}:{}:{}:{}\"", 
                t.query, t.width, t.height, t.x, t.y, 
                if t.is_primary { 1 } else { 0 }, rot, t.freq
            )
        }).collect::<Vec<_>>().join(" ")
    }

    fn write_batch_payload(path: &std::path::Path, tasks: &[DisplayTask], post_cmd: Option<String>) -> io::Result<()> {
        let mut content = format!(
            "@echo off\n\
             start \"\" \"%~dp0screen_animation.exe\" -d left right\n\
             powershell -command \"Start-Sleep -Milliseconds 3200\"\n\
             \"%~dp0displayflow.exe\" {}\n", 
            Self::format_task_args(tasks)
        );
        
        if let Some(cmd) = post_cmd { 
            content.push_str(&format!("start \"\" \"{}\"\n", cmd)); 
        }
        
        fs::write(path, content)
    }

    fn write_vbs_wrapper(vbs_path: &std::path::Path, bat_path: &std::path::Path) -> io::Result<()> {
        let content = format!(
            "Set W = CreateObject(\"WScript.Shell\")\nW.Run \"cmd /c \"\"{}\"\"\", 0", 
            bat_path.display()
        );
        fs::write(vbs_path, content)
    }

    
    pub fn capture_hotkey_physical() -> Option<String> {
        thread::sleep(Duration::from_millis(500));
        loop {
            let mut combo = Vec::new();
            unsafe {
                if GetAsyncKeyState(VK_CONTROL.0 as i32) as u16 & 0x8000 != 0 { combo.push("CTRL".to_string()); }
                if GetAsyncKeyState(VK_MENU.0 as i32) as u16 & 0x8000 != 0 { combo.push("ALT".to_string()); }
                if GetAsyncKeyState(VK_SHIFT.0 as i32) as u16 & 0x8000 != 0 { combo.push("SHIFT".to_string()); }
                
                let mut key_id = None;
                for vk in (0x41..=0x5A).chain(0x70..=0x7B) {
                    if GetAsyncKeyState(vk as i32) as u16 & 0x8000 != 0 {
                        key_id = Some(match vk {
                            0x70..=0x7B => format!("F{}", vk - 0x6F),
                            _ => (vk as u8 as char).to_string(),
                        });
                        break;
                    }
                }
                
                if let Some(key) = key_id {
                    if combo.len() >= 1 {
                        combo.push(key);
                        let result = combo.join("+");
                        while Self::is_any_modifier_down() { thread::sleep(Duration::from_millis(10)); }
                        return Some(result);
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

    fn create_desktop_shortcut(name: &str, vbs: &str, icon: &str, hk: &str) -> io::Result<()> {
        let script = format!(
            "$ws = New-Object -ComObject WScript.Shell; \
             $lnk = $ws.CreateShortcut((Join-Path ([Environment]::GetFolderPath('Desktop')) 'df_{}.lnk')); \
             $lnk.TargetPath = 'wscript.exe'; \
             $lnk.Arguments = '\"\"\"{}\"\"\"'; \
             $lnk.Hotkey = '{}'; \
             if (Test-Path '{}') {{ $lnk.IconLocation = '{}' }}; \
             $lnk.Save();",
            name, vbs, hk, icon, icon
        );

        let out = Command::new("powershell")
            .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", &script])
            .output()?;

        if !out.status.success() {
            return Err(io::Error::new(io::ErrorKind::Other, "PS_EXEC_FAILURE"));
        }
        Ok(())
    }
}