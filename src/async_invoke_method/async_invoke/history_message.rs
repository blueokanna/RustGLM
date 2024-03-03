use serde_json::json;
use std::fs::{self, File};
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

const HISTORY_FILE: &str = "chatglm_history.json";

pub struct HistoryMessage {
    history_file_path: PathBuf,
}

impl HistoryMessage {
    pub fn new() -> Self {
        let history_file_path = PathBuf::from(HISTORY_FILE);
        Self::create_history_file_if_not_exists(&history_file_path);

        HistoryMessage { history_file_path }
    }

    fn create_history_file_if_not_exists(file_path: &Path) {
        if !file_path.exists() {
            if let Err(err) = File::create(file_path) {
                eprintln!("Failed to create history file: {}", err);
            }
        }
    }

    pub fn add_history_to_file(&self, role: &str, content: &str) -> String {
        let json = json!({
            "role": role,
            "content": content,
        });

        if let Err(err) = fs::write(&self.history_file_path, format!("{},\n", json)) {
            eprintln!("Failed to write to history file: {}", err);
        }

        json.to_string()
    }

    pub fn load_history_from_file(&self) -> String {
        if let Ok(file) = File::open(&self.history_file_path) {
            let reader = BufReader::new(file);
            reader.lines().filter_map(Result::ok).collect::<String>()
        } else {
            eprintln!("Failed to open history file for reading");
            String::new()
        }
    }
}
