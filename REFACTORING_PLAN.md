# DisplayFlow CLI Refactoring Plan

## Ziel
Registry-Profil-Storage → Config-Datei-basierte Profile
Große Dateien → Kleinere, logisch integrierte Module

## Changes

### 1. Profile Storage Refactoring (profiles.rs → config.rs)
- **Alt:** `winreg::RegKey` → Registry `Software\DisplayFlow\Suites`
- **Neu:** JSON Config-Dateien in `%LOCALAPPDATA%\DisplayFlow\suites\`
- Dateiname: `{suite_name}.json`
- Format:
  ```json
  {
    "name": "suite_name",
    "tasks": [...],
    "hotkey": "Ctrl+Alt+D",
    "post_cmd": null
  }
  ```

### 2. Modul-Aufteilung
**`display_controller.rs` (456 Zeilen) → Aufteilen in:**
- `config.rs` - Suite-Config-Verwaltung (neu, aus profiles.rs extrahiert)
- `display_apply.rs` - Display-Konfiguration anwenden
- `display_snapshot.rs` - Monitor-Snapshot und Wiederherstellung
- `ddc_control.rs` - DDC/CI Hardware-Steuerung
- `display_controller.rs` - Haupt-Orchestrator (gekürzt)

### 3. Neue Dateistruktur
```
src/
  cli.rs           (~78 Zeilen - unverändert)
  config.rs        (~90 Zeilen - neu aus profiles.rs)
  display_apply.rs (~150 Zeilen - extrahiert)
  display_snapshot.rs (~80 Zeilen - extrahiert)
  ddc_control.rs   (~60 Zeilen - extrahiert)
  display_controller.rs (~120 Zeilen - gekürzt)
  main.rs          (~77 Zeilen - angepasst)
  daemon.rs        (unverändert)
  output.rs        (unverändert)
```

### 4. Config-Verzeichnis
- Pfad: `%LOCALAPPDATA%\DisplayFlow\suites\`
- Jede Suite als eigene JSON-Datei
- Bessere Editierbarkeit, Backup-freundlich