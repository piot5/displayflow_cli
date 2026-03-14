pub mod scan;
pub mod synth;

use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct DisplayRow {
    pub source: String, 
    pub name_id: String, 
    pub resolution: String, 
    pub freq: String,
    pub dpi: String, 
    pub scale_factor: f32,
    pub position_instance: String, 
    pub active: String, 
    pub primary: String,
    pub serial: String, 
    pub size_mm: String, 
    pub persistent_id: u32, 
    pub x: i32, 
    pub y: i32, 
    pub is_primary: bool,
}

impl DisplayRow {
    pub fn to_task(&self) -> DisplayTask {
        DisplayTask {
            query: self.persistent_id.to_string(),
            width: self.resolution.split('x').next().unwrap_or("0").parse().unwrap_or(0),
            height: self.resolution.split('x').nth(1).unwrap_or("0").parse().unwrap_or(0),
            x: self.x,
            y: self.y,
            is_primary: self.is_primary,
            direction: None,
            freq: self.freq.replace("Hz","").trim().parse().unwrap_or(0),
        }
    }
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
pub fn collect_inventory() -> Vec<DisplayRow> { synth::collect_inventory() }