pub mod scan;
pub mod synth;

use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DisplayRow {
    pub source: String,
    pub name_id: String,           // GDI Name (e.g., \\.\DISPLAY1)
    pub resolution: String,
    pub freq: String,
    pub dpi: String,
    pub position_instance: String, // Hardware Device ID
    pub active: String,
    pub primary: String,
    pub serial: String,
    pub size_mm: String,
    pub persistent_id: u32,        // The stable ID for the user
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
}

// Re-exports for backward compatibility with main.rs and engine.rs
pub use scan::set_dpi_awareness;
pub use synth::collect_inventory;