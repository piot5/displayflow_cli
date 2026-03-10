pub mod scan;
pub mod synth;

use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DisplayRow {
    pub source: String,
    pub name_id: String,
    pub resolution: String,
    pub freq: String,
    pub dpi: String,
    pub position_instance: String,
    pub active: String,
    pub primary: String,
    pub serial: String,
    pub size_mm: String,
    pub persistent_id: u32,
}

#[derive(Clone, Debug)]
pub struct DisplayTask {
    pub query: String, 
    pub width: i32, 
    pub height: i32, 
    pub x: i32, 
    pub y: i32, 
    pub is_primary: bool, 
    pub direction: Option<String>,
    pub freq: u32,
}

pub use scan::set_dpi_awareness;
pub use synth::collect_inventory;