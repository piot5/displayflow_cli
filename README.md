# DisplayFlow CLI v1.0.9

[![Download Latest Release](https://img.shields.io/badge/Download-DisplayFlow_v0.9.4-blue?style=for-the-badge&logo=windows)](https://github.com/piot5/displayflow_cli/releases/latest/download/displayflow.zip)

Installation:
Download displayflow.exe from the latest Releases or link above.
Place the executable in a folder included in your system's PATH.

Usage:
displayflow.exe [MonitorConfigs] [Command] [Flags]

Format: ID:Width:Height:X:Y:Primary:Rotation:Frequency
(Run displayflow.exe --scan to find your stable persistent IDs).

Examples

0. Snapshot
captures the live state and saves it to a named profile

displayflow.exe --save Simracer


2. Vertical Stack (Monitor 2 above 1)
Sets up a configuration where Monitor 2 is physically positioned above your primary monitor.

displayflow.exe "1:1920:1080:0:0:1:0:60 2:1920:1080:0:-1080:0:0:60" --save Stacked

2. Gaming Setup (Disable Monitor 2 & Launch Steam)
Disables the second monitor (by setting width/height to 0), bumps the primary to 144Hz, and executes a command after the switch.

displayflow.exe "1:2560:1440:0:0:1:0:144 2:0:0:0:0:0:0:0" --save Gaming --post "start steam://open/main" --hotkey

3. Coding Setup (Portrait Mode + Desktop Shortcut)
Rotates Monitor 2 by 90 degrees, places it to the right of the primary, and creates a desktop shortcut.

displayflow.exe "1:3440:1440:0:0:1:0:144 2:1080:1920:3440:0:0:90:60" --save MultiMonitor --linkdesktop

4. Ultrawide Focus (Direct Link Hotkey)
Saves a suite and creates a Windows shortcut (.lnk) that embeds the hotkey logic directly via the -l h flag.

displayflow.exe "1:3440:1440:0:0:1:0:144" --save Focus --linkdesktop h --hotkey

5. Applying a Suite (Silent Mode)
Applies a previously saved registry suite without any console output.

displayflow.exe --apply-suite Gaming --silent

6. Background Service (Daemon)
Starts the tray-icon service to listen for global hotkeys defined in your suites.

displayflow.exe --daemon

Flags & Commands
| Flag | Description |
|---|---|
| --scan | Lists all connected monitors with IDs, resolutions, and positions. |
| --save <Name> | Saves the current (or provided) config to the Registry. |
| --hotkey | Opens the interceptor to capture a key combo for the suite. |
| --linkdesktop | Creates a .lnk file on your Desktop. Use -l h to bind the hotkey to the link. |
| --post "<CMD>" | Command to be executed after the display settings are applied. |
| --daemon | Runs the background service for tray-icon and hotkey support. |
