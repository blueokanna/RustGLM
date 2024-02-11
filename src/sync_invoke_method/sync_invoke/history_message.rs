use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::Path;

const HISTORY_FILE: &str = "chatglm_history.json";

pub struct HistoryMessage {
    history_file_path: String,
}

impl HistoryMessage {
    pub fn new() -> Self {
        let history_file_path = String::from(HISTORY_FILE);
        Self::create_history_file_if_not_exists(&history_file_path);

        HistoryMessage { history_file_path }
    }

    fn create_history_file_if_not_exists(file_path: &str) {
        let path = Path::new(file_path);

        if !path.exists() {
            if let Err(err) = File::create(file_path) {
                eprintln!("Failed to create history file: {}", err);
            }
        }
    }

    pub fn add_history_to_file(&self, role: &str, content: &str) -> String {
        let json = self.create_json(role, content);

        if let Ok(mut file) = OpenOptions::new().write(true).append(true).open(&self.history_file_path) {
            if let Err(err) = writeln!(file, "{},", json) {
                eprintln!("Failed to write to history file: {}", err);
            }
        } else {
            eprintln!("Failed to open history file for writing");
        }

        json
    }

    fn create_json(&self, role: &str, content: &str) -> String {
        let mut historys = serde_json::Map::new();
        historys.insert(String::from("role"), serde_json::Value::String(role.to_string()));
        historys.insert(String::from("content"), serde_json::Value::String(content.to_string()));

        serde_json::to_string(&serde_json::Value::Object(historys)).unwrap()
    }

    pub fn load_history_from_file(&self) -> String {
        if let Ok(file) = File::open(&self.history_file_path) {
            let reader = BufReader::new(file);
            reader.lines().filter_map(Result::ok).collect()
        } else {
            eprintln!("Failed to open history file for reading");
            String::new()
        }
    }
}