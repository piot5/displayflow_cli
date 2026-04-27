use std::collections::HashMap;
use winreg::enums::*;
use winreg::RegKey;
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::Foundation::LPARAM;
use crate::scraper::{DisplayRow, scan, ddc};
use log::{info, debug};

const REG_PATH: &str = r"Software\DisplayFlow\Mapping";


/// Merges live GDI data with static Registry data and DDC
pub fn collect_inventory() -> Vec<DisplayRow> {
    let live_data = scan::scan_gdi_live().unwrap_or_default();
    let registry_data = scan::scan_registry_monitors();
    
    // Fix: Explizite Typ-Annotation für den Enum-Callback
    let mut ddc_map: Vec<(HMONITOR, ddc::DdcCaps)> = Vec::new();
    unsafe {
        // Fix: Cast auf isize statt i_isize
        let _ = EnumDisplayMonitors(None, None, Some(ddc::monitor_enum_proc), LPARAM(&mut ddc_map as *mut _ as isize));
    }

    let mut mapping = load_mapping();
    let mut mapping_changed = false;
    let mut final_results = Vec::new();
    let mut next_id = mapping.values().max().map_or(1, |&max| max + 1);

    for mut synth in live_data {
        if synth.position_instance.is_empty() { continue; }
        synth.source = "synth_data".into();
        
        if let Some(reg) = find_registry_match(&synth.position_instance, &registry_data) {
            synth.serial = reg.serial.clone();
            synth.size_mm = reg.size_mm.clone();
        }

        // Solution #1: Handle missing serial path gracefully (generic drivers)
        // If serial is still "N/A" after registry lookup, create a fallback identifier
        if synth.serial == "N/A" || synth.serial.is_empty() {
            let fallback_id = create_fallback_identifier(&synth);
            debug!("Monitor {} missing serial path, using fallback identifier: {}", synth.name_id, fallback_id);
            synth.serial = fallback_id;
        }

        // DDC Daten zuweisen
        for (h_mon, caps) in &ddc_map {
            let mut info = MONITORINFO::default();
            info.cbSize = std::mem::size_of::<MONITORINFO>() as u32;
            unsafe {
                if GetMonitorInfoW(*h_mon, &mut info).as_bool() {
                    if synth.x >= info.rcMonitor.left && synth.x < info.rcMonitor.right &&
                       synth.y >= info.rcMonitor.top && synth.y < info.rcMonitor.bottom {
                        synth.ddc = Some(caps.clone());
                    }
                }
            }
        }

// Generate keys for ID persistence (Precedence: Path+Serial > Path).
        let (path_key, precise_key) = generate_keys(&synth);
        synth.persistent_id = determine_id(&mut mapping, &path_key, &precise_key, &mut next_id, &mut mapping_changed);
        final_results.push(synth);
    }
// Persist mapping changes to HKCU if any new IDs were assigned or upgraded
    if mapping_changed { save_mapping(&mapping); }
    final_results
}

/// Solution #1: Create a fallback identifier when serial path is missing
/// Hashes name_id + resolution + frequency to create a unique, stable identifier
/// This works around generic driver limitations that don't provide serial paths
fn create_fallback_identifier(row: &DisplayRow) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    
    // Combine multiple attributes for uniqueness
    row.name_id.hash(&mut hasher);
    row.resolution.hash(&mut hasher);
    row.freq.hash(&mut hasher);
    
    let hash = hasher.finish();
    
    // Format as recognizable identifier: "FALLBACK_<hex_hash>"
    format!("FALLBACK_{:016x}", hash)
}

/// Matches a live hardware path against registry keys using case-insensitive substring search.
fn find_registry_match<'a>(hw_path: &str, registry: &'a [DisplayRow]) -> Option<&'a DisplayRow> {
    let uc_path = hw_path.to_uppercase();
    registry.iter().find(|r| {
        let rid = r.name_id.to_uppercase();
        !rid.is_empty() && uc_path.contains(&rid)
    })
}

/// Creates a hierarchy of keys for identification.
/// Prioritizes Path+Serial > Path for stable monitor identification
fn generate_keys(row: &DisplayRow) -> (String, String) {
    let path_key = row.position_instance.clone();
    
    // Solution #1: Accept fallback identifiers as valid serial data
    let precise_key = if (row.serial != "N/A" && !row.serial.is_empty()) || row.serial.starts_with("FALLBACK_") { 
        format!("{}#{}", path_key, row.serial) 
    } else { 
        path_key.clone() 
    };
    (path_key, precise_key)
}

/// Logic to retrieve or create a unique persistent ID.
/// Automatically upgrades "Path-only" keys to "Path+Serial" keys if a serial becomes available.
fn determine_id(mapping: &mut HashMap<String, u32>, path_key: &str, precise_key: &str, next_id: &mut u32, changed: &mut bool) -> u32 {
    if let Some(&id) = mapping.get(precise_key) { 
        return id; 
    }
    if let Some(&id) = mapping.get(path_key) {
        if precise_key != path_key {
            let id_val = id;
            mapping.remove(path_key);
            mapping.insert(precise_key.to_string(), id_val);
            info!("Upgraded monitor mapping from path-only to path+serial/fallback");
            *changed = true;
        }
        return id;
    }
    let id = *next_id;
    mapping.insert(precise_key.to_string(), id);
    *next_id += 1;
    *changed = true;
    id
}

fn load_mapping() -> HashMap<String, u32> {
    let mut map = HashMap::new();
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    if let Ok(key) = hkcu.open_subkey(REG_PATH) {
        for (name, value) in key.enum_values().map_while(std::result::Result::ok) {
            if value.vtype == REG_DWORD {
                if let Ok(bytes) = value.bytes.try_into() {
                    map.insert(name, u32::from_ne_bytes(bytes));
                }
            }
        }
    }
    map
}

fn save_mapping(map: &HashMap<String, u32>) {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    if let Ok((key, _)) = hkcu.create_subkey(REG_PATH) {
        for (name, &id) in map { let _ = key.set_value(name, &id); }
    }
}
