## Header

* **Project**: DisplayFlow CLI
* **Version**: v0.9.2 (Final documentation build)
* **Objective**: Comprehensive technical manual including the `post:exe` automation layer.

---

## Full Code: Complete README.md

```markdown
# DisplayFlow CLI v0.9.2

A high-performance Rust tool for deterministic Windows display management. Unlike standard Windows settings, DisplayFlow maps volatile GDI handles to persistent hardware EDID data, ensuring your layouts remain stable across reboots and standby cycles.

##  Key Features
- **Persistent Mapping**: Links display settings to unique hardware IDs, preventing "scrambled" layouts.
- **Atomic Commits**: Swaps resolution, position, and orientation in a single Win32 GDI cycle.
- **Headless Deployment**: Generates silent VBS wrappers and injects global Windows Hotkeys (e.g., `CTRL+ALT+F1`) directly into your shortcuts.
- **Post-Execution Trigger**: Automatically launches an application (Game, IDE, Browser) after the display switch is complete.
- **Privacy-First Telemetry**: Anonymous hardware fingerprinting (SHA-256) to improve driver compatibility.

##  Installation
1. Download the latest `displayflow.exe` from the **Releases** or **Actions/Artifacts** tab.
2. Move the `.exe` to a permanent folder (e.g., `C:\Tools\`).
3. (Optional) Add the folder to your Windows System PATH.

##  Detailed Parameter Reference
The CLI follows a deterministic argument structure: `[Monitor Configurations] [Command] [Flags]`.

### Monitor String Format
`ID:Width:Height:X:Y:Primary:Rotation`
- **ID**: Persistent hardware ID (found via `displayflow.exe` without params).
- **Width/Height**: Resolution in pixels. Use `0:0` to disable the display.
- **X/Y**: Desktop coordinates. `0:0` is the top-left of the primary monitor.
- **Primary**: `1` for the main display, `0` for secondary.
- **Rotation**: `0` (None), `1` (90°), `2` (180°), `3` (270°).

### Commands & Flags
- `save:Name`: Saves the profile and creates a silent VBS execution wrapper.
- `post:"C:\Path\To\App.exe"`: Executes the specified program immediately after the display change.
- `--hotkey`: (Used with `save`) Prompts for a physical key combo to bind to the Windows shortcut.
- `-t` / `--telemetry`: Opt-in to send an anonymous success/error report to the developer database.

##  Usage Examples

### 1. Vertical Stack (Monitor 2 above Monitor 1)
```bash
# Monitor 1: 1080p (Primary)
# Monitor 2: 1080p (Above Monitor 1 at Y=-1080)
displayflow.exe "1:1920:1080:0:0:1:0 2:1920:1080:0:-1080:0:0" -t save:Stacked

```

### 2. Gaming Mode with Auto-Launch

```bash
# Set primary to 1440p, disable others, and launch Steam
displayflow.exe "1:2560:1440:0:0:1:0 2:0:0:0:0:0:0" -t save:Gaming post:"C:\Program Files (x86)\Steam\steam.exe" --hotkey

```

### 3. Professional Coding Setup (Portrait Mode)

```bash
# Monitor 1: Ultrawide Landscape
# Monitor 2: 1080p Portrait (Rotated 90°)
displayflow.exe "1:3440:1440:0:0:1:0 2:1080:1920:3440:0:0:1" save:Dev -t

```

##  Telemetry for Improvement  -t param

This tool includes an opt-in telemetry system to identify failing Win32 GDI calls on specific hardware.

* **Data**: Anonymized hardware hashes and configuration success states.
* **Security**: Transmitted via Supabase using encrypted secrets injected during build.

##  Built With

* **Rust & Windows-RS**: For memory-safe Win32 API interactions.
* **GitHub Actions**: For secure, automated binary releases.

**Would you like me to help you set up a "Releases" description on GitHub that uses this new documentation as a template?**

```
