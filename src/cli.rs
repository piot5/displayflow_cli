use clap::{Parser, ArgAction};
use crate::scraper::DisplayTask;

#[derive(Parser, Debug)]
#[command(author, version, about = "DisplayFlow CLI")]
pub struct Cli {
    #[arg(short, long, action = ArgAction::SetTrue)] pub silent: bool,
    #[arg(short, long, action = ArgAction::SetTrue)] pub hotkey: bool,
    #[arg(short, long, action = ArgAction::SetTrue)] pub scan: bool,
    #[arg(short, long, action = ArgAction::SetTrue)] pub daemon: bool,
    #[arg(short('l'), long, num_args(0..=1), default_missing_value = "plain")] pub linkdesktop: Option<String>,
    #[arg(short, long)] pub clone: Vec<String>,
    #[arg(long)] pub save: Option<String>,
    #[arg(long)] pub post: Option<String>,
    #[arg(long)] pub apply_suite: Option<String>,
    #[arg(value_name = "TASKS")] pub tasks: Vec<String>,
    #[arg(long)] pub animation: Option<String>,
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
        brightness: p.get(8).and_then(|&s| s.parse().ok()),
        contrast: p.get(9).and_then(|&s| s.parse().ok()),
        animation: p.get(10).filter(|&&s| s != "0").map(|&s| s.to_string()),
    })
}