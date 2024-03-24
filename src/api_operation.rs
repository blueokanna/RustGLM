use std::error::Error;
use std::fs;
use std::fs::{File, OpenOptions};
use once_cell::sync::OnceCell;
use std::io::{BufRead, BufReader, Seek, SeekFrom, Write};
use serde_derive::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Serialize, Deserialize, Debug)]
struct ChatApiConfig {
    api_key: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
struct AiConfig {
    chatglm_api_key: Vec<ChatApiConfig>,
}

pub async fn chatglm_api_read_config(file_path: &str, glm: &str) -> Result<String, Box<dyn Error>> {
    let file_content = tokio::fs::read_to_string(file_path).await?;
    let config: AiConfig = toml::from_str(&file_content)?;

    let response = match glm {
        "chatglm_api_key" => config.chatglm_api_key,
        _ => return Err("Invalid ChatGLM API".into()),
    };

    let json_string = serde_json::to_string(&response)?;

    Ok(json_string)
}

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

    pub async fn load_api_key(user_config: &str) -> Result<String, Box<dyn Error>> {
        let json_string = match chatglm_api_read_config(user_config, "chatglm_api_key").await {
            Ok(final_json_string) => final_json_string,
            Err(err) => return Err(format!("Error reading config file: {}", err).into()),
        };

        let api_key: Value = serde_json::from_str(&json_string)
            .map_err(|err| format!("Failed to parse JSON: {}", err))?;

        let glm_key = api_key[0]["api_key"]
            .as_str()
            .ok_or_else(|| "Failed to get api_key")?
            .to_string();

        Ok(glm_key)
    }

    pub fn save_api_key(user_config: &str, api_key: &str) -> Result<(), Box<dyn std::error::Error>> {
        let config = if let Ok(contents) = fs::read_to_string(user_config) {
            toml::from_str::<AiConfig>(&contents)?
        } else {
            AiConfig {
                chatglm_api_key: Vec::new(),
            }
        };

        if config.chatglm_api_key.iter().any(|c| c.api_key.as_ref().map(|k| k == api_key).unwrap_or(false)) {
            println!("API key already exists. Skipping...");
            return Ok(());
        }

        ChatApiConfig {
            api_key: Some(api_key.to_string()),
        };

        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(user_config)?;
        if let Some(pos) = Self::find_insert_position(&mut file, "[[chatglm_api_key]]")? {
            file.seek(SeekFrom::Start(pos))?;
        } else {
            file.seek(SeekFrom::End(0))?;
            //writeln!(file, "[[chatglm_api_key]]")?;

        }
        writeln!(file, "[[chatglm_api_key]]")?;
        writeln!(file, "api_key = \"{}\"", api_key)?;

        Ok(())
    }

    fn find_insert_position(file: &mut File, target: &str) -> Result<Option<u64>, Box<dyn Error>> {
        let reader = BufReader::new(file);
        let mut pos = 0;
        let mut found_target = false;
        for line in reader.lines() {
            let line = line?;
            if line.starts_with(target) {
                found_target = true;
                continue; // Continue searching for the next line after the target
            }
            if found_target {
                return Ok(Some(pos));
            }
            pos += line.len() as u64 + 1; // Add 1 for the newline character
        }
        Ok(None)
    }
}