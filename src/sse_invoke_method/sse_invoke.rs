mod history_message;
mod constant_value;

use std::collections::VecDeque;
use std::error::Error;
use std::io::Read;
use regex::Regex;
use reqwest;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use futures::stream::StreamExt;
use futures_util::stream::iter;
use crate::sse_invoke_method::sse_invoke::constant_value::{LANGUAGE_MODEL, SYSTEM_CONTENT, SYSTEM_ROLE, USER_ROLE, TEMP_FLOAT, TOP_P_FLOAT, ASSISTANT_ROLE};

lazy_static::lazy_static! {
    static ref UNICODE_REGEX: regex::Regex = regex::Regex::new(r"\\u[0-9a-fA-F]{4}").unwrap();
}

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
pub struct SSEInvokeModel {
    get_message: String,
    ai_response_data : String,
}

impl SSEInvokeModel {
    pub fn new() -> Self {
        SSEInvokeModel {
            get_message: String::new(),
            ai_response_data : String::new(),
        }
    }

    pub async fn sse_request(token: String, input: String, default_url: String) -> Result<String, Box<dyn Error>> {
        let mut sse_invoke_model = Self::new();
        Self::sse_invoke_request_method(&mut sse_invoke_model, token.clone(), input.clone(), default_url.clone()).await?;
        let mut response_message = sse_invoke_model.ai_response_data.clone();
        let result = sse_invoke_model.process_sse_message(&*response_message, &input);
        Ok(result)
    }

    async fn generate_sse_json_request_body(
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
        "stream": true,
        "do_sample":true,
        "temperature": temp_float,
        "top_p": top_p_float
    });

        let json_string = serde_json::to_string(&json_request_body)?;

        // 替换字符，注意使用转义符号
        let result = json_string.replace(r"\\\\", r"\\").replace(r"\\", r"").trim().to_string();

        // 打印生成的 JSON 字符串
        //println!("{:#}", result.trim());

        Ok(result)
    }

    pub async fn sse_invoke_request_method(
        &mut self,
        token: String,
        user_input: String,
        default_url: String,
    ) -> Result<String, String> {
        let json_content = match Self::generate_sse_json_request_body(
            LANGUAGE_MODEL,
            SYSTEM_ROLE,
            SYSTEM_CONTENT.trim(),
            USER_ROLE,
            &*user_input,
            TEMP_FLOAT,
            TOP_P_FLOAT,
        )
            .await
        {
            Ok(result) => result.to_string(),
            Err(err) => return Err(err.to_string()),
        };

        let request_result = reqwest::Client::new()
            .post(&default_url)
            .header("Cache-Control", "no-cache")
            .header("Connection", "keep-alive")
            .header("Accept", "text/event-stream")
            .header("Content-Type", "application/json;charset=UTF-8")
            .header("Authorization", format!("Bearer {}", token))
            .body(json_content)
            .send()
            .await
            .map_err(|err| format!("HTTP request failure: {}", err))?;

        if !request_result.status().is_success() {
            return Err(format!("Server returned an error: {}", request_result.status()));
        }

        // Create an async reader for the response body
        let mut response_body = request_result.bytes_stream();
        // 用于存储 SSE 事件的字符串
        let mut sse_data = String::new();

        // 处理 SSE 事件
        while let Some(chunk) = response_body.next().await {
            match chunk {
                Ok(bytes) => {
                    let data = String::from_utf8_lossy(&bytes);
                    sse_data.push_str(&data);
                    self.ai_response_data = sse_data.clone();
                    //println!("{}",self.ai_response_data.clone());

                    /*
                    let parts: Vec<&str> = sse_data.split(':').collect();
                    if parts.len() == 2 && parts[0].trim() == "data" {
                        if let Some(json_content) = parts.get(1) {
                            //sse_queue.push_back(json_content.trim().to_string());

                        }
                    }
                    */

                    if data.contains("data: [DONE]") {
                        break;
                    }
                }
                Err(e) => {
                    return Err(format!("Error receiving SSE event: {}", e));
                }
            }
        }

        Ok(sse_data)
    }

    fn process_sse_message(&mut self, response_data: &str, user_message: &str) -> String {
        let mut char_queue = VecDeque::new();
        let mut queue_result = String::new();

        let json_messages: Vec<&str> = response_data.lines()
            .map(|line| line.trim_start_matches("data: "))
            .filter(|line| !line.is_empty())
            .collect();

        for json_message in json_messages {
            if json_message.trim() == "[DONE]" {
                break;
            }

            if let Ok(json_element) = serde_json::from_str::<Value>(json_message) {
                if let Some(json_response) = json_element.as_object() {
                    if let Some(choices) = json_response.get("choices").and_then(Value::as_array) {
                        if let Some(choice) = choices.get(0).and_then(Value::as_object) {
                            if let Some(delta) = choice.get("delta").and_then(Value::as_object) {
                                if let Some(content) = delta.get("content").and_then(Value::as_str) {
                                    let get_message = self.convert_unicode_emojis(content)
                                        .replace("\"", "")
                                        .replace("\\n\\n", "\n")
                                        .replace("\\nn", "\n")
                                        .replace("\\\\n", "\n")
                                        .replace("\\\\nn", "\n")
                                        .replace("\\", "");

                                    for c in get_message.chars() {
                                        char_queue.push_back(c);
                                    }
                                }
                            }
                        }
                    }
                } else {
                    println!("Invalid JSON format: {:?}", json_element);
                }
            } else {
                println!("Error reading JSON: {}", json_message);
            }
        }

        queue_result.extend(char_queue);


        if !queue_result.is_empty() {
            let message_process = history_message::HistoryMessage::new();
            message_process.add_history_to_file(USER_ROLE, user_message);
            message_process.add_history_to_file(ASSISTANT_ROLE, &*queue_result);
        }

        queue_result
    }




    fn convert_unicode_emojis(&self, input: &str) -> String {
        UNICODE_REGEX.replace_all(input, |caps: &regex::Captures| {
            let emoji = char::from_u32(
                u32::from_str_radix(&caps[0][2..], 16).expect("Failed to parse Unicode escape"),
            )
                .expect("Invalid Unicode escape");
            emoji.to_string()
        })
            .to_string()
    }

    pub fn response_sse_message(&self) -> &str {
        &self.get_message
    }
}
