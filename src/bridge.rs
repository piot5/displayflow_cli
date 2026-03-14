use std::io::{self, Write};
use serde::Serialize;

pub enum OutMode { Console, Pipe }

pub struct IOBridge { pub mode: OutMode }

impl IOBridge {
    pub fn new(is_daemon: bool) -> Self {
        Self { mode: if is_daemon { OutMode::Pipe } else { OutMode::Console } }
    }

    pub fn output<T: Serialize>(&self, tag: &str, data: &T) {
        match self.mode {
            OutMode::Console => {
                let json = serde_json::to_string_pretty(data).unwrap_or_default();
                println!("[{}] {}", tag, json);
            }
            OutMode::Pipe => {
                if let Ok(json) = serde_json::to_string(data) {
                    let mut stdout = io::stdout();
                    let _ = writeln!(stdout, "{}:{}", tag, json);
                    let _ = stdout.flush();
                }
            }
        }
    }
    
    #[allow(dead_code)]
    pub fn log(&self, msg: &str) {
        if let OutMode::Console = self.mode { println!("> {}", msg); }
    }
}