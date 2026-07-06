use crate::scraper::{DisplayRow, scan};
use crate::scraper::ddc;
use log::{info, debug};

/// Merges live GDI data with static Registry data and DDC (updated to use df_ddc)
pub fn collect_inventory() -> Vec<DisplayRow> {
    let live_data = scan::scan_gdi_live().unwrap_or_default();
    let registry_data = scan::scan_registry_monitors();

    // Use df_ddc-based caps list instead of EnumDisplayMonitors callback
    let ddc_list = ddc::list_ddc_caps(); // Vec<(info_string, DdcCaps)>

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

        // If serial missing, create fallback identifier
        if synth.serial == "N/A" || synth.serial.is_empty() {
            let fallback_id = create_fallback_identifier(&synth);
            debug!("Monitor {} missing serial path, using fallback identifier: {}", synth.name_id, fallback_id);
            synth.serial = fallback_id;
        }

        // DDC Daten zuweisen: best-effort match using device info string
        for (info, caps) in &ddc_list {
            if synth.name_id.contains(info) || synth.position_instance.contains(info) || synth.serial.contains(info) {
                synth.ddc = Some(caps.clone());
                break;
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
