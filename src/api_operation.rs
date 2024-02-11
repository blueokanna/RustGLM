use once_cell::sync::OnceCell;
use std::fs::File;
use std::io::{BufRead, BufReader,Write};

const API_KEY_FILE: &str = "chatglm_api_key.txt";

pub struct APIKeys {
    user_id: String,
    user_secret: String,
}

impl APIKeys {
    fn new(user_id: &str, user_secret: &str) -> APIKeys {
        APIKeys {
            user_id: user_id.to_string(),
            user_secret: user_secret.to_string(),
        }
    }

    pub fn get_instance(api: &str) -> &APIKeys {
        static INSTANCE: OnceCell<APIKeys> = OnceCell::new();

        INSTANCE.get_or_init(|| {
            let parts: Vec<&str> = api.trim().split('.').collect();
            if parts.len() == 2 {
                APIKeys::new(parts[0], parts[1])
            } else {
                panic!("Your API Key is Invalid");
            }
        })
    }

    pub fn get_user_id(&self) -> &str {
        &self.user_id
    }

    pub fn get_user_secret(&self) -> &str {
        &self.user_secret
    }

    pub fn load_api_key() -> Option<String> {
        match File::open(API_KEY_FILE) {
            Ok(file) => {
                let reader = BufReader::new(file);
                reader.lines().next().map(|line| line.unwrap_or_default())
            }
            Err(_) => None,
        }
    }

    pub fn save_api_key(api_key: &str) {
        match File::create(API_KEY_FILE) {
            Ok(mut file) => {
                if let Err(e) = writeln!(file, "{}", api_key) {
                    eprintln!("Failed to write to file: {}", e);
                }
            }
            Err(e) => eprintln!("Failed to create file: {}", e),
        }
    }
}
