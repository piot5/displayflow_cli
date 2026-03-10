# DisplayFlow CLI v0.9.4

[![Download Latest Release](https://img.shields.io/badge/Download-DisplayFlow_v0.9.4-blue?style=for-the-badge&logo=windows)](https://github.com/piot5/displayflow_cli/releases/latest/download/displayflow.zip)

Quickly switch and save Windows display layouts via CLI


## Installation
1. Download `displayflow.exe` from Releases.
2. Put it somewhere in your PATH.


## Usage
`displayflow.exe [MonitorConfigs] [Command] [Flags]`

**Format:** `ID:Width:Height:X:Y:Primary:Rotation:Frequenz`
(Run without params or doubleclick .exe to see your IDs)


## Examples
### Vertical Stack (Monitor 2 above 1)
```bash
displayflow.exe "1:1920:1080:0:0:1:0:60 2:1920:1080:0:-1080:0:0:60" save:Stacked
```

### Gaming (Disable Monitor 2 & launch Steam)
```bash
displayflow.exe "1:2560:1440:0:0:1:0:120 2:0:0:0:0:0:0:0" save:Gaming post:"C:\Path\To\Steam.exe" --hotkey
```

### Coding Setup (Portrait Mode)
```bash
# Monitor 1: Ultrawide Landscape
# Monitor 2: 1080p Portrait (Rotated 90°)
displayflow.exe "1:3440:1440:0:0:1:0:144 2:1080:1920:3440:0:0:1:144" save:Dev
```


