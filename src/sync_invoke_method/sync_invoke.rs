mod history_message;
mod constant_value;

use std::error::Error;
use regex::Regex;
use reqwest;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use futures_util::stream::StreamExt;
use crate::sync_invoke_method::sync_invoke::constant_value::{LANGUAGE_MODEL, SYSTEM_CONTENT, SYSTEM_ROLE, USER_ROLE, TEMP_FLOAT, TOP_P_FLOAT, ASSISTANT_ROLE};

pub struct MessageProcessor {
    messages: history_message::HistoryMessage,
    user_role: String,
}

impl MessageProcessor {
    pub fn new(user_role: &str) -> Self {
        MessageProcessor {
            messages: history_message::HistoryMessage::new(),
            user_role: user_role.to_string(),
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

    pub fn last_messages(&self, role:&str, messages: &str) -> String {
        let input_message = self.set_input_message().unwrap_or_default();

        let mut input: Value = serde_json::from_str(&input_message).unwrap_or_default();
        input["role"] = Value::String(role.to_string());
        input["content"] = Value::String(messages.to_string());

        let texts = serde_json::to_string(&input).unwrap_or_default();

        let regex = Regex::new(r",(\s*})").expect("Failed to create regex pattern");

        let user_messages = (input_message.clone() + &texts.clone());
        let result = regex.replace_all(&user_messages, "");

        result.to_string()
    }
}


#[derive(Debug, Serialize, Deserialize)]
pub struct SyncInvokeModel {
    get_message: String,
    ai_response_data : String,
}

impl SyncInvokeModel {
    pub fn new() -> Self {
        SyncInvokeModel {
            get_message: String::new(),
            ai_response_data : String::new(),
        }
    }

    pub async fn sync_request(token: String, input: String, default_url: String) -> Result<String, Box<dyn Error>> {
        let mut sync_invoke_model = Self::new();
        Self::sync_invoke_request_method(&mut sync_invoke_model, token.clone(), input.clone(), default_url.clone()).await?;
        let mut response_message = sync_invoke_model.ai_response_data.clone();
        let result = sync_invoke_model.process_sync_task_status(&*response_message, &input);
        Ok(result)
    }

    async fn generate_json_request_body(
        language_model: &str,
        system_role: &str,
        system_content: &str,
        user_role: &str,
        user_input: &str,
        temp_float: f64,
        top_p_float: f64,
    ) -> Result<String, Box<dyn Error>> {

        let message_process = MessageProcessor::new(user_role);

        let messages = json!([
        {"role": system_role, "content": system_content},
        {"role": user_role, "content": message_process.last_messages(user_role,user_input)}
    ]);

        let json_request_body = json!({
        "model": language_model,
        "messages": messages,
        "stream": false,
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
        default_url: String,
    ) -> Result<String, String> {
        let json_content = match Self::generate_json_request_body(LANGUAGE_MODEL, SYSTEM_ROLE, SYSTEM_CONTENT.trim(), USER_ROLE, &*user_input, TEMP_FLOAT, TOP_P_FLOAT).await {
            Ok(result) => result.to_string(),
            Err(err) => return Err(err.to_string()),
        };

        let request_result = reqwest::Client::new()
            .post(&default_url)
            .header("Accept", "application/json")
            .header("Content-Type", "application/json;charset=UTF-8")
            .header("Authorization", format!("Bearer {}", token))
            .body(json_content)
            .send()
            .await
            .map_err(|err| format!("HTTP request failure: {}", err))?;

        if !request_result.status().is_success() {
            return Err(format!("Server returned an error: {}", request_result.status()));
        }

        let response_text = request_result.text().await.map_err(|err| format!("Failed to read response text: {}", err))?;
        self.ai_response_data = response_text.clone();

        Ok(response_text)
    }

    fn process_sync_task_status(&mut self, response_data: &str, user_input: &str) -> String{
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
                message_process.add_history_to_file(USER_ROLE,user_input);
                message_process.add_history_to_file(ASSISTANT_ROLE,&*self.get_message);


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
    pub fn get_content_message(&self) -> &str {
        &self.get_message
    }
}
