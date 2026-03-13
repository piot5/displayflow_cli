use std::collections::HashMap;
use winreg::enums::*;
use winreg::RegKey;
use crate::scraper::{DisplayRow, scan};

const REG_PATH: &str = r"Software\DisplayFlow\Mapping";

pub fn collect_inventory() -> Vec<DisplayRow> {
    let live_data = scan::scan_gdi_live().unwrap_or_default();
    let registry_data = scan::scan_registry_monitors();
    let mut mapping = load_mapping();
    let mut mapping_changed = false;
    let mut final_results = Vec::new();
    let mut next_id = mapping.values().max().map_or(1, |&max| max + 1);

    for mut synth in live_data {
        if synth.position_instance.is_empty() { continue; }
        synth.source = "Integrated_Final".into();
        if let Some(reg) = find_registry_match(&synth.position_instance, &registry_data) {
            synth.serial = reg.serial.clone();
            synth.size_mm = reg.size_mm.clone();
        }
        let (path_key, precise_key) = generate_keys(&synth);
        synth.persistent_id = determine_id(&mut mapping, &path_key, &precise_key, &mut next_id, &mut mapping_changed);
        final_results.push(synth);
    }

    if mapping_changed { save_mapping(&mapping); }
    final_results
}

fn find_registry_match<'a>(hw_path: &str, registry: &'a [DisplayRow]) -> Option<&'a DisplayRow> {
    let uc_path = hw_path.to_uppercase();
    registry.iter().find(|r| {
        let rid = r.name_id.to_uppercase();
        !rid.is_empty() && uc_path.contains(&rid)
    })
}

fn generate_keys(row: &DisplayRow) -> (String, String) {
    let path_key = row.position_instance.clone();
    let precise_key = if row.serial != "N/A" && !row.serial.is_empty() { format!("{}#{}", path_key, row.serial) } else { path_key.clone() };
    (path_key, precise_key)
}

fn determine_id(mapping: &mut HashMap<String, u32>, path_key: &str, precise_key: &str, next_id: &mut u32, changed: &mut bool) -> u32 {
    if let Some(&id) = mapping.get(precise_key) { return id; }
    if let Some(&id) = mapping.get(path_key) {
        if precise_key != path_key {
            let id_val = id;
            mapping.remove(path_key);
            mapping.insert(precise_key.to_string(), id_val);
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
        for (name, value) in key.enum_values().map_while(Result::ok) {
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