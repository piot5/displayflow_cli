use std::{fs, collections::HashMap, path::Path};
use crate::scraper::{DisplayRow, scan};

pub fn collect_inventory() -> Vec<DisplayRow> {
    let live_data = scan::scan_gdi_live();
    let registry_data = scan::scan_registry_monitors();
    let mut mapping = load_mapping();
    let mut mapping_changed = false;
    let mut final_results = Vec::new();

    let mut next_id = mapping.values().max().cloned().unwrap_or(0) + 1;

    for live_row in live_data {
        if live_row.position_instance.is_empty() {
            continue;
        }

        let mut synth = live_row.clone();
        synth.source = "Integrated_Final".into();
        let gdi_hw_path = synth.position_instance.to_uppercase();

        
        if let Some(reg) = registry_data.iter().find(|r| {
            let rid = r.name_id.to_uppercase();
            !rid.is_empty() && gdi_hw_path.contains(&rid)
        }) {
            synth.serial = reg.serial.clone();
            synth.size_mm = reg.size_mm.clone();
        }

       
        let path_key = synth.position_instance.clone();
        let serial_suffix = if synth.serial != "N/A" && !synth.serial.is_empty() {
            format!("#{}", synth.serial)
        } else {
            String::new()
        };
        let precise_key = format!("{}{}", path_key, serial_suffix);

        
        let persistent_id = if let Some(&id) = mapping.get(&precise_key) {
            id
        } else if let Some(&id) = mapping.get(&path_key) {
            if !serial_suffix.is_empty() {
                mapping.remove(&path_key); 
                mapping.insert(precise_key.clone(), id);
                mapping_changed = true;
            }
            id
        } else {
            
            let id = next_id;
            mapping.insert(precise_key.clone(), id);
            next_id += 1;
            mapping_changed = true;
            id
        };

        synth.persistent_id = persistent_id;
        final_results.push(synth);
    }

    if mapping_changed {
        save_mapping(&mapping);
    }
    final_results
}

fn load_mapping() -> HashMap<String, u32> {
    Path::new("mapping.json").exists()
        .then(|| fs::read_to_string("mapping.json").ok())
        .flatten()
        .and_then(|c| serde_json::from_str(&c).ok())
        .unwrap_or_default()
}

fn save_mapping(map: &HashMap<String, u32>) {
    let clean_map: HashMap<String, u32> = map.iter()
        .filter(|(k, _)| !k.is_empty())
        .map(|(k, v)| (k.clone(), *v))
        .collect();

    if let Ok(json) = serde_json::to_string_pretty(&clean_map) {
        let _ = fs::write("mapping.json", json);
    }
}