use std::{fs, collections::HashMap, path::Path};
use crate::scraper::{DisplayRow, scan};

const MAPPING_FILE: &str = "monitor_db.json";

pub fn collect_inventory() -> Vec<DisplayRow> {
    let live_data = scan::scan_gdi_live();
    let registry_data = scan::scan_registry_monitors();
    let mut mapping = load_mapping();
    let mut mapping_changed = false;
    let mut final_results = Vec::new();

    let mut next_id = mapping.values().max().map_or(1, |&max| max + 1);

    for live_row in live_data {
        if live_row.position_instance.is_empty() { continue; }

        let mut synth = live_row.clone();
        synth.source = "Integrated_Final".into();
        
        
        if let Some(reg) = find_registry_match(&synth.position_instance, &registry_data) {
            synth.serial = reg.serial.clone();
            synth.size_mm = reg.size_mm.clone();
        }

        
        let (path_key, precise_key) = generate_keys(&synth);
        let persistent_id = determine_id(
            &mut mapping, 
            &path_key, 
            &precise_key, 
            &mut next_id, 
            &mut mapping_changed
        );

        synth.persistent_id = persistent_id;
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
    let precise_key = if row.serial != "N/A" && !row.serial.is_empty() {
        format!("{}#{}", path_key, row.serial)
    } else {
        path_key.clone()
    };
    (path_key, precise_key)
}

fn determine_id(
    mapping: &mut HashMap<String, u32>,
    path_key: &str,
    precise_key: &str,
    next_id: &mut u32,
    changed: &mut bool
) -> u32 {
    if let Some(&id) = mapping.get(precise_key) {
        return id;
    }

    if let Some(&id) = mapping.get(path_key) {
        if precise_key != path_key {
            mapping.remove(path_key);
            mapping.insert(precise_key.to_string(), id);
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
    let path = Path::new(MAPPING_FILE);
    if !path.exists() { return HashMap::new(); }

    
    fs::read_to_string(path)
        .and_then(|content| {
            serde_json::from_str(&content)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
        })
        .unwrap_or_else(|e: std::io::Error| {
            eprintln!("ERR_LOAD_MAPPING: {}", e);
            HashMap::new()
        })
}

fn save_mapping(map: &HashMap<String, u32>) {
    let clean_map: HashMap<String, u32> = map.iter()
        .filter(|(k, _)| !k.is_empty())
        .map(|(k, v)| (k.clone(), *v))
        .collect();

    match serde_json::to_string_pretty(&clean_map) {
        Ok(json) => {
            if let Err(e) = fs::write(MAPPING_FILE, json) {
                eprintln!("ERR_WRITE_MAPPING: {}", e);
            }
        },
        Err(e) => eprintln!("ERR_SERIALIZE_MAPPING: {}", e),
    }
}