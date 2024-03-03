mod history_message;

extern crate toml;

use std::io::prelude::*;
use std::error::Error;
use std::fs::File;
use regex::Regex;
use reqwest;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Serialize, Deserialize, Debug)]
struct AiResponse {
    language_model: Option<String>,
    system_role: Option<String>,
    system_content: Option<String>,
    user_role: Option<String>,
    assistant_role: Option<String>,
    max_tokens: Option<f64>,
    temp_float: Option<f64>,
    top_p_float: Option<f64>,
}

#[derive(Serialize, Deserialize, Debug)]
struct AiConfig {
    ai_config_glm3: Vec<AiResponse>,
    ai_config_glm4: Vec<AiResponse>,
}

fn sync_read_config(file_path: &str, glm: &str) -> Result<String, Box<dyn Error>> {
    let mut file = File::open(file_path)?;
    let mut file_content = String::new();
    file.read_to_string(&mut file_content)?;

    let config: AiConfig = toml::from_str(&file_content)?;

    let response = match glm {
        "glm-3" => config.ai_config_glm3,
        "glm-4" => config.ai_config_glm4,
        _ => return Err(Box::from("Invalid glm")),
    };

    // 将 AiResponse 向量转换为 JSON 字符串
    let json_string = serde_json::to_string(&response)?;

    Ok(json_string)
}

pub struct MessageProcessor {
    messages: history_message::HistoryMessage,
}

impl MessageProcessor {
    pub fn new() -> Self {
        MessageProcessor {
            messages: history_message::HistoryMessage::new(),
        }
    }

    pub fn set_input_message(&self) -> Option<String> {
        let message = self.messages.load_history_from_file();
        if !message.is_empty() {
            Some(message)
        } else {
            None
        }
    }

    pub fn last_messages(&self, role: &str, messages: &str) -> String {
        let input_message = self.set_input_message().unwrap_or_default();

        let mut input: Value = serde_json::from_str(&input_message).unwrap_or_default();
        input["role"] = Value::String(role.to_string());
        input["content"] = Value::String(messages.to_string());

        let texts = serde_json::to_string(&input).unwrap_or_default();

        let regex = Regex::new(r",(\s*})").expect("Failed to create regex pattern");

        let user_messages = input_message.clone() + &texts.clone();
        let result = regex.replace_all(&user_messages, "");

        result.to_string()
    }
}


#[derive(Debug, Serialize, Deserialize)]
pub struct SyncInvokeModel {
    get_message: String,
    ai_response_data: String,
}

impl SyncInvokeModel {
    pub fn new() -> Self {
        SyncInvokeModel {
            get_message: String::new(),
            ai_response_data: String::new(),
        }
    }

    pub async fn sync_request(token: String, input: String, user_config: &str, default_url: String) -> Result<String, Box<dyn Error>> {
        let mut sync_invoke_model = Self::new();
        Self::sync_invoke_request_method(&mut sync_invoke_model, token.clone(), input.clone(), user_config, default_url.clone()).await?;
        let response_message = sync_invoke_model.ai_response_data.clone();
        let result = sync_invoke_model.process_sync_task_status(&*response_message, &input);
        Ok(result)
    }

    async fn generate_sync_json_request_body(
        language_model: &str,
        system_role: &str,
        system_content: &str,
        user_role: &str,
        user_input: &str,
        max_token: f64,
        temp_float: f64,
        top_p_float: f64,
    ) -> Result<String, Box<dyn Error>> {
        let message_process = MessageProcessor::new();

        let messages = json!([
        {"role": system_role, "content": system_content},
        {"role": user_role, "content": message_process.last_messages(user_role,user_input)}
    ]);

        let json_request_body = json!({
        "model": language_model,
        "messages": messages,
        "stream": false,
        "max_tokens": max_token,
        "temperature": temp_float,
        "top_p": top_p_float
    });

        let json_string = serde_json::to_string(&json_request_body)?;
        let result = json_string.replace(r"\\\\", r"\\").replace(r"\\", r"").trim().to_string();

        Ok(result)
    }

    pub async fn sync_invoke_request_method(
        &mut self,
        token: String,
        user_input: String,
        user_config: &str,
        default_url: String,
    ) -> Result<String, String> {
        let json_string = match crate::sync_invoke_method::sync_invoke::sync_read_config(user_config, "glm-4") {
            Ok(json_string) => json_string,
            Err(err) => return Err(format!("Error reading config file: {}", err)),
        };

        let json_value: Value = serde_json::from_str(&json_string)
            .expect("Failed to parse Toml to JSON");

        let language_model = json_value[0]["language_model"]
            .as_str().expect("Failed to get language_model").to_string();

        let system_role = json_value[0]["system_role"]
            .as_str().expect("Failed to get system_role").to_string();

        let system_content = json_value[0]["system_content"]
            .as_str().expect("Failed to get system_content").to_string().trim().to_string();

        let user_role = json_value[0]["user_role"]
            .as_str().expect("Failed to get user_role").to_string();

        let max_token = json_value[0]["max_tokens"]
            .as_f64().expect("Failed to get max_token");

        let temp_float = json_value[0]["temp_float"]
            .as_f64().expect("Failed to get temp_float");

        let top_p_float = json_value[0]["top_p_float"]
            .as_f64().expect("Failed to get top_p_float");

        let user_json_content = match Self::generate_sync_json_request_body(
            &language_model,
            &system_role,
            &system_content,
            &user_role,
            &user_input,
            max_token,
            temp_float,
            top_p_float,
        ).await {
            Ok(result) => result.to_string(),
            Err(err) => return Err(err.to_string()),
        };

        let request_result = reqwest::Client::new()
            .post(&default_url)
            .header("Accept", "application/json")
            .header("Content-Type", "application/json;charset=UTF-8")
            .header("Authorization", format!("Bearer {}", token))
            .body(user_json_content)
            .send()
            .await
            .map_err(|err| format!("HTTP request failure: {}", err))?;

        if !request_result.status().is_success() {
            return Err(format!("Server returned an error: {}", request_result.status()));
        }

        //println!("Sync Calling Now!");


        let response_text = request_result.text().await.map_err(|err| format!("Failed to read response text: {}", err))?;
        self.ai_response_data = response_text.clone();

        Ok(response_text)
    }

    fn process_sync_task_status(&mut self, response_data: &str, user_input: &str) -> String {
        let result = serde_json::from_str::<Value>(response_data)
            .map_err(|e| format!("Error processing response data: {}", e))
            .and_then(|json_response| {
                if let Some(choices) = json_response.get("choices").and_then(|c| c.as_array()) {
                    if let Some(choice) = choices.get(0).and_then(|c| c.as_object()) {
                        if let Some(message) = choice.get("message").and_then(|m| m.as_object()) {
                            if let Some(content) = message.get("content").and_then(|c| c.as_str()) {
                                Ok(content.to_string())
                            } else {
                                Err("Content not found in message".to_string())
                            }
                        } else {
                            Err("Message not found in choice".to_string())
                        }
                    } else {
                        Err("Choice not found in choices".to_string())
                    }
                } else {
                    Err("Choices not found in response".to_string())
                }
            });

        match result {
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

                //self.get_message.(USER_ROLE, );
                //self.get_message.add_history_to_file(ASSISTANT_ROLE, &self.get_message);
                let message_process = history_message::HistoryMessage::new();
                message_process.add_history_to_file("user", user_input);
                message_process.add_history_to_file("assistant", &*self.get_message);


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
