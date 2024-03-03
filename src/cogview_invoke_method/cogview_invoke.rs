extern crate toml;

use std::error::Error;
use std::fs::File;
use std::io::prelude::*;
use serde_derive::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Serialize, Deserialize, Debug)]
struct CogView {
    model: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
struct CogViewConfig3 {
    cogview_config_3: Vec<CogView>,
}

fn cogview_read_config(file_path: &str, glm: &str) -> Result<String, Box<dyn Error>> {
    let mut file = File::open(file_path)?;
    let mut file_content = String::new();
    file.read_to_string(&mut file_content)?;

    let config: CogViewConfig3 = toml::from_str(&file_content)?;

    let response = match glm {
        "cogview-3" => config.cogview_config_3,
        _ => return Err(Box::from("Invalid glm")),
    };

    let json_string = serde_json::to_string(&response)?;

    Ok(json_string)
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CogViewInvokeModel {
    get_message: String,
    ai_response_data: String,
}

impl CogViewInvokeModel {
    pub fn new() -> Self {
        CogViewInvokeModel {
            get_message: String::new(),
            ai_response_data: String::new(),
        }
    }

    pub async fn cogview_request(token: String, input: String, user_config:&str, default_url: String) -> Result<String, Box<dyn Error>> {
        let mut cogview_invoke_model = Self::new();
        Self::cogview_invoke_method(&mut cogview_invoke_model, token.clone(), input.clone(), user_config, default_url.clone()).await?;
        let response_message = cogview_invoke_model.ai_response_data.clone();
        let result = cogview_invoke_model.process_cogview_task_status(&*response_message);
        Ok(result)
    }

    async fn generate_cogview_request_body(
        model: &str,
        user_input: &str,
    ) -> Result<String, Box<dyn Error>> {
        let json_request_body = json!({
        "model": model,
        "prompt": user_input,
    });

        let json_string = serde_json::to_string(&json_request_body)?;
        let result = json_string.replace(r"\\\\", r"\\").replace(r"\\", r"").trim().to_string();

        Ok(result)
    }

    pub async fn cogview_invoke_method(
        &mut self,
        token: String,
        user_input: String,
        config_file : &str,
        default_url: String,
    ) -> Result<String, String> {
        let json_string = match cogview_read_config(config_file, "cogview-3") {
            Ok(json_string) => json_string,
            Err(err) => return Err(format!("Error reading config file: {}", err)),
        };

        let json_value: Value = serde_json::from_str(&json_string)
            .expect("Failed to parse Toml to JSON");

        let model = json_value[0]["model"]
            .as_str().expect("Failed to get cogview_model").to_string();


        let cogview_json_content = match Self::generate_cogview_request_body(
            &model,
            &user_input,
        ).await {
            Ok(result) => result.to_string(),
            Err(err) => return Err(err.to_string()),
        };

        let cogview_request_result = reqwest::Client::new()
            .post(&default_url)
            .header("Accept", "application/json")
            .header("Content-Type", "application/json;charset=UTF-8")
            .header("Authorization", format!("Bearer {}", token))
            .body(cogview_json_content)
            .send()
            .await
            .map_err(|err| format!("HTTP request failure: {}", err))?;

        if !cogview_request_result.status().is_success() {
            return Err(format!("Server returned an error: {}", cogview_request_result.status()));
        }

        let response_text = cogview_request_result.text().await.map_err(|err| format!("Failed to read response text or url: {}", err))?;
        self.ai_response_data = response_text.clone();

        Ok(response_text)
    }

    fn process_cogview_task_status(&mut self, response_data: &str) -> String {
        let cogview_result = serde_json::from_str::<Value>(response_data)
            .map_err(|e| format!("Error processing response data: {}", e))
            .and_then(|json_response| {
                if let Some(cogview_data) = json_response.get("data").and_then(|c| c.as_array()) {
                    if let Some(image_url) = cogview_data.get(0).and_then(|c| c.as_object()) {
                        if let Some(url) = image_url.get("url").and_then(|c| c.as_str()) {
                            Ok(url.to_string())
                        } else {
                            Err("ImageUrl not found in message".to_string())
                        }
                    } else {
                        Err("url not found in data part".to_string())
                    }
                } else {
                    Err("data part not found in response".to_string())
                }
            });

        match cogview_result {
            Ok(content) => {
                self.get_message = self.convert_unicode_emojis(&content);
                self.get_message = self.get_message
                    .replace("\"", "")
                    .replace("\\n\\n", "\n")
                    .replace("\\nn\\nn", "\n")
                    .replace("\\\\nn", "\n")
                    .replace("\\n", "\n")
                    .replace("\\nn", "\n")
                    .replace("\\\\", "");

                self.get_message.clone()
            }
            Err(e) => {
                eprintln!("{}", e);
                String::new()
            }
        }
    }

    fn convert_unicode_emojis(&self, input: &str) -> String {
        let regex = regex::Regex::new(r"\\u[0-9a-fA-F]{4}").unwrap();
        let result = regex.replace_all(input, |caps: &regex::Captures| {
            let emoji = char::from_u32(
                u32::from_str_radix(&caps[0][2..], 16).expect("Failed to parse Unicode escape"),
            )
                .expect("Invalid Unicode escape");
            emoji.to_string()
        });
        result.to_string()
    }
}