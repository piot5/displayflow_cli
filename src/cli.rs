use clap::{Parser, ArgAction};
use crate::scraper::DisplayTask;

#[derive(Parser, Debug)]
#[command(
    author = "DisplayFlow Team",
    version = "0.1.2",
    about = "DisplayFlow: Windows Display & DDC/CI Management",
    long_about = "A CLI utility to manage monitor layouts, resolutions, and hardware settings like brightness. \
                  Settings can be saved as 'Suites' in the registry and automated via hotkeys.")]
pub struct Cli {
    #[arg(short, long, action = ArgAction::SetTrue, help = "Suppress console output")]
    pub silent: bool,

    #[arg(short, long, action = ArgAction::SetTrue, help = "Enable hotkey recording when saving a suite(profile)")]
    pub hotkey: bool,

    #[arg(short, long, action = ArgAction::SetTrue, help = "Scan and list all connected displays and their current IDs")]
    pub scan: bool,

    #[arg(short, long, action = ArgAction::SetTrue, help = "Start the background daemon hotkey listener and tray icon")]
    pub daemon: bool,

    #[arg(
        short('l'), 
        long, 
        num_args(0..=1), 
        default_missing_value = "plain", 
        help = "Create a desktop shortcut for a suite"
    )]
    pub linkdesktop: Option<String>,

    #[arg(short, long, help = "Clone settings from one display to another (IDs)")]
    pub clone: Vec<String>,

    #[arg(long, help = "Save the current or specified setup as a named Suite")]
    pub save: Option<String>,

    #[arg(long, help = "Command to execute automatically after applying settings")]
    pub post: Option<String>,

    #[arg(long, help = "Apply a previously saved Suite by name")]
    pub apply_suite: Option<String>,

    #[arg(
        value_name = "TASKS", 
        help = "Manual configuration string. Format: 'ID:W:H:X:Y:Primary:Rot:Freq:Bright:Cont:Direction(of animated transition)'"
    )]
    pub tasks: Vec<String>,

    #[arg(long, help = "Transition animation type direction(up, down, left, right)")]
    pub animation: Option<String>,
}

pub fn parse_task(raw: &str) -> Option<DisplayTask> {
    let p: Vec<&str> = raw.split(':').collect();
    if p.len() < 5 { return None; }

    Some(DisplayTask {
        query: p[0].into(),
        width: p[1].parse().ok()?,
        height: p[2].parse().ok()?,
        x: p[3].parse().ok()?,
        y: p[4].parse().ok()?,
        is_primary: p.get(5).map(|&s| s == "1").unwrap_or(false),
        direction: p.get(6).filter(|&&s| s != "0").map(|&s| s.to_string()),
        freq: p.get(7).and_then(|&s| s.parse().ok()).unwrap_or(0),
        brightness: p.get(8).and_then(|&s| {
            let val = s.parse::<u32>().unwrap_or(999);
            if val <= 100 { Some(val) } else { None }
        }),
        contrast: p.get(9).and_then(|&s| {
            let val = s.parse::<u32>().unwrap_or(999);
            if val <= 100 { Some(val) } else { None }
        }),
        animation: p.get(10).filter(|&&s| s != "0").map(|&s| s.to_string()),
    })
}