Markdown

# DisplayFlow CLI v0.9.2

A high-performance Rust tool for deterministic Windows display management. Unlike standard Windows settings, DisplayFlow maps volatile GDI handles to persistent hardware EDID data, ensuring your layouts remain stable across reboots and standby cycles.

##  Key Features
- **Persistent Mapping**: Links display settings to unique hardware IDs, preventing "scrambled" layouts.
- **Atomic Commits**: Swaps resolution, position, and orientation in a single Win32 GDI cycle.
- **Seamless Deployment**: Generates silent VBS wrappers and injects global Windows Hotkeys (e.g., `CTRL+ALT+F1`) directly into your shortcuts.
- **Privacy-First Telemetry**: Anonymous hardware fingerprinting (SHA-256) to improve driver compatibility across different GPU/Monitor setups.

##  Installation
1. Download the latest `displayflow.exe` from the **Releases** or **Actions/Artifacts** tab.
2. Move the `.exe` to a permanent folder (e.g., `C:\Tools\`).
3. (Optional) Add the folder to your Windows System PATH.

##  Usage Examples

### 1. Identify Your Displays
Get a list of all connected monitors and their persistent IDs:

displayflow.exe (no params. list is in same folder as exe mapping.json ) 

2. Save a "Gaming" Profile with Hotkey

Set your primary monitor to 144Hz and center it, then bind it to a Hotkey:
Bash

displayflow.exe "1:2560:1440:0:0:1:0" save:Gaming --hotkey

The tool will prompt you to press a key combination (e.g., Ctrl+Alt+G) to create a desktop shortcut.
3. Dual-Head Office Setup
Bash

displayflow.exe "1:1920:1080:0:0:1:0 2:1920:1080:1920:0:0:0" save:Office

 Telemetry for Improvement

This tool includes an opt-in telemetry system.

    What we collect: Anonymized hardware IDs (CPU/RAM hashes) and display configurations.

    Why: To identify failing Win32 GDI calls on specific hardware combinations and improve the engine.rs logic.

    Security: Data is transmitted securely via Supabase using encrypted compile-time secrets.

 Built With

    Rust: For memory-safe, zero-cost abstractions over the Win32 API.

    Windows-RS: Official Rust bindings for Windows APIs.

    GitHub Actions: Automated CI/CD for secure binary builds.
