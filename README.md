# DisplayFlow v0.1.2

![Banner](https://github.com/user-attachments/assets/4b1b31bb-8ece-4f0a-8a0e-12ae368e489f)

**DisplayFlow** is a high-performance, Rust-based CLI utility for Windows 10/11 designed to manage complex multi-monitor setups. Unlike standard Windows settings, DisplayFlow allows for granular control over resolutions, positions, refresh rates, and hardware-level monitor settings (DDC/CI) like brightness and contrast—all via the command line or global hotkeys.

[![Download Latest Release](https://img.shields.io/badge/Download-DisplayFlow-blue?style=for-the-badge&logo=windows)](https://github.com/piot5/displayflow_cli/releases/latest/download/displayflow.zip)

---

## 🚀 Key Features

* **Snapshot Technology:** Save your current desktop arrangement (positions, Hz, brightness) into a named "Suite" with a single command.
* **DDC/CI Integration:** Control hardware monitor brightness and contrast directly without touching monitor buttons.
* **Zero-Delay Switching:** Apply complex layout changes, including primary monitor swaps and rotations, instantly.
* **Background Daemon:** A lightweight service that sits in the system tray and listens for global hotkeys to switch profiles.
* **Automation Ready:** Generate desktop shortcuts or batch scripts to trigger specific display environments.

---

## 🛠 Quick Start

### 1. Create a Snapshot
Arrange your monitors manually in Windows Settings, then save the state:
```powershell
./displayflow.exe --save Gaming --hotkey
```
*The `--hotkey` flag will prompt you to press a key combination (e.g., Alt+Shift+G) to bind to this profile.*

### 2. Scan your Hardware
To see persistent IDs and current hardware values (DDC):
```powershell
./displayflow.exe --scan
```

### 3. Apply a Saved Suite
```powershell
./displayflow.exe --apply-suite Gaming
```

---

## 📝 Task Format (Manual Configuration)

For scripting or manual overrides, use the colon-separated string format:

`ID : Width : Height : X : Y : Primary : Rotation : Freq : Brightness : Contrast : [Animation]`

| Field | Description |
| :--- | :--- |
| **ID** | Persistent Monitor ID (e.g., BNQ78A7 or 1). Find via `--scan`. |
| **Width/Height** | Resolution in pixels (e.g., 1920:1080). Use 0:0 to "Soft-Disable". |
| **X/Y** | Desktop coordinates. (0:0 is usually the top-left of the primary). |
| **Primary** | `1` for Yes, `0` for No. |
| **Rotation** | `0` (0°), `1` (90°), `2` (180°), `3` (270°). |
| **Freq** | Refresh rate in Hz (e.g., 144). |
| **Brightness** | DDC/CI Hardware brightness (0-100). |
| **Contrast** | DDC/CI Hardware contrast (0-100). |
| **Animation** | Transition direction (`up`, `down`, `left`, `right`). |

---

## 💡 Advanced Examples

### 1. Complex Layout with Transitions
Set up two monitors with different brightness levels and an animated transition.
```powershell
./displayflow.exe "1:1920:1080:0:0:1:0:60:60:80:down" "2:1920:1080:0:1080:0:0:60:70:90:up" --save Production
```

### 2. High-Performance Gaming Mode
Force the primary display to 144Hz, max brightness, and launch a post-execution command.
```powershell
./displayflow.exe "1:2560:1440:0:0:1:0:144:100:80" --save GamingHigh --post "start steam://open/main"
```

---

## ⌨️ Command Reference

| Flag | Effect |
| :--- | :--- |
| `--scan` | Lists all monitor IDs, capabilities, and current values. |
| `--save <Name>` | Saves the current live state (or provided task string) as a Suite. |
| `--apply-suite <Name>` | Restores a previously saved layout from the registry. |
| `--hotkey` | Records a global shortcut (e.g., Ctrl+Alt+1) for the suite. |
| `--daemon` | Starts the background listener for hotkeys and adds a tray icon. |
| `--linkdesktop` | Creates a `.lnk` shortcut on your desktop to trigger a suite. |
| `--silent` | Suppresses all console output (ideal for automation/scripts). |


---
*Created by the DisplayFlow Team. Licensed under MIT.*
```