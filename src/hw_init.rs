use std::{mem, iter};
use windows::core::PCWSTR;
use windows::Win32::Graphics::Gdi::{
    EnumDisplaySettingsW, ChangeDisplaySettingsExW, DEVMODEW, ENUM_CURRENT_SETTINGS,
    CDS_UPDATEREGISTRY, CDS_NORESET, DM_PELSWIDTH, DM_PELSHEIGHT, 
    DM_POSITION, DM_DISPLAYORIENTATION, CDS_TYPE, ENUM_REGISTRY_SETTINGS
};
use windows::Win32::Devices::Display::{SetDisplayConfig, SDC_APPLY, SDC_TOPOLOGY_EXTEND};
use crate::scraper::{self, DisplayRow};

/// Sorgt dafür, dass temporär aktivierte Monitore nach dem Scan wieder 
/// in ihren ursprünglichen Zustand (aus/an) versetzt werden.
struct DisplayGuard {
    snapshot: Vec<(String, bool, DEVMODEW)>, 
}

impl Drop for DisplayGuard {
    fn drop(&mut self) {
        for (name_id, was_active, old_dm) in &self.snapshot {
            let name_u16: Vec<u16> = name_id.encode_utf16().chain(iter::once(0)).collect();
            
            if !*was_active {
                // Monitor war vorher aus -> Wieder deaktivieren
                let mut detach_dm: DEVMODEW = unsafe { mem::zeroed() };
                detach_dm.dmSize = mem::size_of::<DEVMODEW>() as u16;
                detach_dm.dmPelsWidth = 0;
                detach_dm.dmPelsHeight = 0;
                detach_dm.dmFields = DM_PELSWIDTH | DM_PELSHEIGHT | DM_POSITION;
                unsafe {
                    let _ = ChangeDisplaySettingsExW(
                        PCWSTR(name_u16.as_ptr()), 
                        Some(&detach_dm), 
                        None, 
                        CDS_UPDATEREGISTRY | CDS_NORESET, 
                        None
                    );
                }
            } else {
                // Monitor war vorher an -> Ursprüngliche Settings (Position/Rotation) wiederherstellen
                let mut reset_dm = old_dm.clone();
                reset_dm.dmFields |= DM_PELSWIDTH | DM_PELSHEIGHT | DM_POSITION | DM_DISPLAYORIENTATION;
                unsafe {
                    let _ = ChangeDisplaySettingsExW(
                        PCWSTR(name_u16.as_ptr()), 
                        Some(&reset_dm), 
                        None, 
                        CDS_UPDATEREGISTRY | CDS_NORESET, 
                        None
                    );
                }
            }
        }
        // Abschließender Flush der Registry-Änderungen an das System
        unsafe { 
            ChangeDisplaySettingsExW(None, None, None, CDS_TYPE(0), None); 
            let _ = SetDisplayConfig(None, None, SDC_APPLY);
        }
    }
}

/// Der zentrale Workflow:
/// 1. Erfasst den aktuellen Hardware-Zustand.
/// 2. Aktiviert alle bekannten Monitore (Staging), damit Windows IDs zuweist.
/// 3. Scannt die Hardware final ab.
/// 4. `DisplayGuard` stellt den Zustand im Hintergrund wieder her.
pub fn run_discovery_flow() -> Vec<DisplayRow> {
    // Erster Scan: Was ist überhaupt vorhanden (auch inaktive)?
    let inventory_initial = scraper::collect_inventory();
    let mut snapshot_data = Vec::new();

    for row in &inventory_initial {
        let mut dm: DEVMODEW = unsafe { mem::zeroed() };
        dm.dmSize = mem::size_of::<DEVMODEW>() as u16;
        let name_u16: Vec<u16> = row.name_id.encode_utf16().chain(iter::once(0)).collect();
        
        // Versuche Registry-Settings zu lesen, falls Monitor aktuell offline ist
        unsafe {
            if !EnumDisplaySettingsW(PCWSTR(name_u16.as_ptr()), ENUM_REGISTRY_SETTINGS, &mut dm).as_bool() {
                EnumDisplaySettingsW(PCWSTR(name_u16.as_ptr()), ENUM_CURRENT_SETTINGS, &mut dm);
            }
        }
        
        let is_active = row.active.to_uppercase() == "YES";
        snapshot_data.push((row.name_id.clone(), is_active, dm));
    }
    
    // Starte den Discovery-Block
    {
        // Guard speichert Zustand; am Ende des Blocks (Drop) wird aufgeräumt
        let _guard = DisplayGuard { snapshot: snapshot_data };

        // Schritt A: Windows zwingen, die Topologie zu erweitern (weckt schlafende Ausgänge)
        unsafe { let _ = SetDisplayConfig(None, None, SDC_TOPOLOGY_EXTEND); }

        // Schritt B: Alle inaktiven Monitore auf eine Standard-Auflösung setzen (Staging)
        for row in &inventory_initial {
            if row.active.to_uppercase() != "YES" {
                let name_u16: Vec<u16> = row.name_id.encode_utf16().chain(iter::once(0)).collect();
                let mut temp_dm: DEVMODEW = unsafe { mem::zeroed() };
                temp_dm.dmSize = mem::size_of::<DEVMODEW>() as u16;
                temp_dm.dmPelsWidth = 1920;
                temp_dm.dmPelsHeight = 1080;
                temp_dm.dmFields = DM_PELSWIDTH | DM_PELSHEIGHT;
                
                unsafe {
                    let _ = ChangeDisplaySettingsExW(
                        PCWSTR(name_u16.as_ptr()), 
                        Some(&temp_dm), 
                        None, 
                        CDS_UPDATEREGISTRY | CDS_NORESET, 
                        None
                    );
                }
            }
        }

        // Schritt C: Registry flashen, damit Scraper echte Pfade sieht
        unsafe { ChangeDisplaySettingsExW(None, None, None, CDS_TYPE(0), None); }

        // Schritt D: Der eigentliche finale Hardware-Scan
        // Da wir uns noch im Scope des Guards befinden, sind die Monitore "logisch aktiv"
        scraper::collect_inventory()
    } 
    // Hier wird _guard gedroppt -> Monitore gehen zurück in den Originalzustand
}