use std::thread;
use std::process;
use anyhow::{Result, anyhow};
use winreg::enums::*;
use winreg::RegKey;
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::Win32::UI::Input::KeyboardAndMouse::{RegisterHotKey, MOD_CONTROL, MOD_ALT, MOD_SHIFT, MOD_NOREPEAT};
use windows::Win32::UI::Shell::*;
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM, POINT, GetLastError};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use crate::engine::DFEngine;
use crate::apply_registry_suite;

const WM_TRAY_ICON: u32 = WM_USER + 1;
const TRAY_ICON_ID: u32 = 1;
const IDM_EXIT: usize = 1001;

static mut GLOBAL_ENGINE: Option<DFEngine> = None;

pub fn start_daemon_service(engine: DFEngine) -> Result<()> {
    unsafe { GLOBAL_ENGINE = Some(engine); }
    ctrlc::set_handler(move || { process::exit(0); }).expect("Error setting Ctrl-C handler");

    unsafe {
        let instance = GetModuleHandleW(None)?;
        let class_name = windows::core::w!("DisplayFlowDaemon");
        let wc = WNDCLASSW {
            lpfnWndProc: Some(window_proc),
            hInstance: instance.into(),
            lpszClassName: class_name,
            ..Default::default()
        };
        RegisterClassW(&wc);

        let hwnd = CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            class_name,
            windows::core::w!("DF_Daemon"),
            WS_OVERLAPPED,
            0, 0, 0, 0,
            None, None, instance, None
        );

        if hwnd.0 == 0 { return Err(anyhow!("Window creation failed")); }

        setup_tray_icon(hwnd);
        register_daemon_hotkeys(hwnd);

        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).as_bool() {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
    Ok(())
}

unsafe fn register_daemon_hotkeys(hwnd: HWND) {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    if let Ok(key) = hkcu.open_subkey(r"Software\DisplayFlow\Suites") {
        for (i, (name, value)) in key.enum_values().map_while(Result::ok).enumerate() {
            let raw_val: String = value.to_string();
            if let Some(hk_part) = raw_val.split(';').find(|p| p.starts_with("hotkey|")) {
                let combo = hk_part.split('|').nth(1).unwrap_or("");
                let mut modifiers = MOD_NOREPEAT;
                let mut key_code = 0u32;

                for part in combo.split('+') {
                    match part.to_uppercase().as_str() {
                        "CTRL" => modifiers |= MOD_CONTROL,
                        "ALT" => modifiers |= MOD_ALT,
                        "SHIFT" => modifiers |= MOD_SHIFT,
                        s if s.len() == 1 => key_code = s.chars().next().unwrap() as u32,
                        _ => {}
                    }
                }

                if key_code != 0 {
                    if let Err(_) = RegisterHotKey(hwnd, i as i32, modifiers, key_code) {
                        let err = GetLastError();
                        eprintln!("\n[CONFLICT] Hotkey '{}' for suite '{}' is already in use (Error: {:?}).", combo, name, err);
                        eprintln!("[CAUSE] This is usually caused by an existing Windows Desktop Shortcut (.lnk) using the same key.");
                        eprintln!("[INSTRUCTION] To allow the Daemon to handle this hotkey:");
                        eprintln!(" 1. Find 'df_{}.lnk' on your Desktop.", name);
                        eprintln!(" 2. Right-click -> Properties -> Shortcut tab.");
                        eprintln!(" 3. Set 'Shortcut key' to None or delete the shortcut.");
                        eprintln!(" 4. Restart the DisplayFlow Daemon.\n");
                    } else {
                        println!("[SUCCESS] Registered daemon hotkey '{}' for '{}'", combo, name);
                    }
                }
            }
        }
    }
}

unsafe fn setup_tray_icon(hwnd: HWND) {
    let mut nid = NOTIFYICONDATAW {
        cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
        hWnd: hwnd,
        uID: TRAY_ICON_ID,
        uFlags: NIF_ICON | NIF_MESSAGE | NIF_TIP,
        uCallbackMessage: WM_TRAY_ICON,
        ..Default::default()
    };
    let icon_path = windows::core::w!("DF.ico");
    let h_icon = LoadImageW(None, icon_path, IMAGE_ICON, 0, 0, LR_LOADFROMFILE | LR_DEFAULTSIZE);
    nid.hIcon = if let Ok(h) = h_icon { HICON(h.0) } else { LoadIconW(None, IDI_APPLICATION).unwrap() };
    
    let tooltip = "DisplayFlow Daemon Active";
    let mut tooltip_wide = [0u16; 128];
    for (i, c) in tooltip.encode_utf16().enumerate().take(127) { tooltip_wide[i] = c; }
    nid.szTip = tooltip_wide;
    let _ = Shell_NotifyIconW(NIM_ADD, &nid);
}

extern "system" fn window_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    unsafe {
        match msg {
            WM_TRAY_ICON if lparam.0 as u32 == WM_RBUTTONUP => {
                let mut cursor = POINT::default();
                let _ = GetCursorPos(&mut cursor);
                if let Ok(h_menu) = CreatePopupMenu() {
                    let _ = AppendMenuW(h_menu, MF_STRING, IDM_EXIT, windows::core::w!("Exit DisplayFlow"));
                    let _ = SetForegroundWindow(hwnd);
                    let _ = TrackPopupMenu(h_menu, TPM_BOTTOMALIGN | TPM_LEFTALIGN, cursor.x, cursor.y, 0, hwnd, None);
                    let _ = DestroyMenu(h_menu);
                }
                LRESULT(0)
            }
            WM_COMMAND if wparam.0 == IDM_EXIT => { 
                let _ = DestroyWindow(hwnd); 
                LRESULT(0) 
            }
            WM_HOTKEY => {
                let idx = wparam.0;
                thread::spawn(move || {
                    if let Some(ref engine) = GLOBAL_ENGINE {
                        let _ = apply_registry_suite_by_index(idx as usize, engine);
                    }
                });
                LRESULT(0)
            }
            WM_DESTROY => {
                let nid = NOTIFYICONDATAW { 
                    cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32, 
                    hWnd: hwnd, 
                    uID: TRAY_ICON_ID, 
                    ..Default::default() 
                };
                let _ = Shell_NotifyIconW(NIM_DELETE, &nid);
                PostQuitMessage(0);
                LRESULT(0)
            }
            _ => DefWindowProcW(hwnd, msg, wparam, lparam),
        }
    }
}

fn apply_registry_suite_by_index(idx: usize, engine: &DFEngine) -> Result<()> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let key = hkcu.open_subkey(r"Software\DisplayFlow\Suites")?;
    let mut names = Vec::new();
    for (name, _) in key.enum_values().map_while(Result::ok) {
        names.push(name);
    }
    if let Some(name) = names.get(idx) {
        apply_registry_suite(name, engine, true)?;
    }
    Ok(())
}