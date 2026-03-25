# DisplayFlow CLI v1.1.0

[![Download Latest Release](https://img.shields.io/badge/Download-DisplayFlow_v0.9.4-blue?style=for-the-badge&logo=windows)](https://github.com/piot5/displayflow_cli/releases/latest/download/displayflow.zip)

Quick Start: The "Snapshot" Feature

The easiest way to use DisplayFlow is a Snapshot. If you provide --save without a configuration string, the tool reads your current live setup and saves it exactly as it is.
PowerShell

# 1. Manually arrange your windows/monitors in Windows Settings
# 2. Run this to save the current state as "Gaming"
displayflow.exe --save Gaming --hotkey

Comprehensive Task Format

For manual configuration or scripting, use the following colon-separated string format. Note that the Animation Direction is defined per monitor at the very end of its string.

Format:
ID : Width : Height : X : Y : Primary : Rotation : Freq : Brightness : Contrast : [Animation]
Field	Description
ID	Persistent ID (e.g., BNQ78A7 or 1). Find via --scan.
Primary	1 for primary monitor, 0 for secondary.
Rotation	0 (Normal), 90 (Portrait), 180 (Inverted), 270 (Portrait Flipped).
Brightness	Hardware level 0-100 (via DDC/CI).
Animation	Direction for screen_animation.exe (up, down, left, right, none).

Advanced Examples
1. Per-Monitor Animation (The "Split" Transition)

If you have a vertical setup, you can trigger animations in opposite directions for a seamless transition effect.
PowerShell

# Top monitor slides 'down', Bottom monitor slides 'up'
displayflow.exe "1:1920:1080:0:0:1:0:60:60:80:down 2:1920:1080:0:1080:0:0:60:70:90:up" --save Production

2. The "Ghost" Monitor (Soft Disable)

Completely disable a monitor by setting its dimensions to zero while keeping its ID in the config.
PowerShell

displayflow.exe "2:0:0:0:0:0" --save FocusMode

3. High-Performance Gaming (144Hz + Full Brightness)

Force the primary display to its max refresh rate and hardware brightness.
PowerShell

displayflow.exe "1:2560:1440:0:0:1:0:144:100:80" --save HighFPS --post "start steam://open/main"


Command Reference
Flag	Effect
--scan	Crucial: Lists all IDs and current hardware values.
--save <Name>	Saves the provided string (or current live state if string is missing).
--apply-suite <Name>	Instantly restores a saved layout.
--hotkey	Records a global shortcut (e.g., Alt+Shift+G) for the suite.
--daemon	Starts the background service for Hotkeys and Tray-Icon.
--silent	No console output (perfect for .bat or .ps1 scripts).

How it works

    Zero Admin: Runs entirely in User Mode. No UAC prompts.

    Persistent IDs: Monitors are identified by their hardware signature (synth.rs), so your settings survive a reboot or GPU port swap.

    Smart Animation: If an animation direction is present in the task string, screen_animation.exe is triggered instantly in a background thread to mask the Windows display-driver handshake.
