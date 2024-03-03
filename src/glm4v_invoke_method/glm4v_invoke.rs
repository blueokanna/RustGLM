extern crate toml;

use std::collections::VecDeque;
use std::error::Error;
use regex::Regex;
use reqwest;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use futures_util::stream::StreamExt;

lazy_static::lazy_static! {
    static ref UNICODE_REGEX: regex::Regex = regex::Regex::new(r"\\u[0-9a-fA-F]{4}").unwrap();
}

#[derive(Serialize, Deserialize, Debug)]
struct Glm4vConfig {
    model: Option<String>,
    user_role: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
struct AiConfig {
    ai_config_glm4v: Vec<Glm4vConfig>,
}

async fn glm4v_read_config(file_path: &str, glm: &str) -> Result<String, Box<dyn Error>> {
    let file_content = tokio::fs::read_to_string(file_path).await?;
    let config: AiConfig = toml::from_str(&file_content)?;

    let response = match glm {
        "glm-4v" => config.ai_config_glm4v,
        _ => return Err("Invalid glm4v".into()),
    };

    let json_string = serde_json::to_string(&response)?;

    Ok(json_string)
}


#[derive(Debug, Serialize, Deserialize)]
pub struct GLM4vInvokeModel {
    ai_response_data: String,
}

#[derive(Serialize, Deserialize)]
struct ImageUrl {
    url: String,
}

#[derive(Serialize, Deserialize)]
struct Content {
    #[serde(rename = "type")]
    content_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    image_url: Option<ImageUrl>,
}

#[derive(Serialize, Deserialize)]
struct JSONResonseData {
    role: String,
    content: Vec<Content>,
}

fn create_json_message(user_role: String, user_input: String) -> JSONResonseData {
    let regex_input = Regex::new(r"([^@]+)@([^@]+)").unwrap();

    let mut part1_content = String::new();
    let mut part2_content = String::new();

    if let Some(captures_content) = regex_input.captures(&user_input) {
        if let Some(first_part) = captures_content.get(1) {
            part1_content = first_part.as_str().to_string();
        }
        if let Some(second_part) = captures_content.get(2) {
            part2_content = second_part.as_str().to_string();
        }
    } else {
        println!("Input does not match the pattern");
    }

    JSONResonseData {
        role: user_role,
        content: vec![
            Content {
                content_type: "text".to_string(),
                text: Some(part1_content),
                image_url: None,
            },
            Content {
                content_type: "image_url".to_string(),
                text: None,
                image_url: Some(ImageUrl { url: part2_content }),
            },
        ],
    }
}

impl GLM4vInvokeModel {
    pub fn new() -> Self {
        GLM4vInvokeModel {
            ai_response_data: String::new(),
        }
    }

    pub async fn glm4v_request(token: String, input: String, user_config: &str, default_url: String) -> Result<String, Box<dyn Error>> {
        let mut glm4v_invoke_model = Self::new();
        Self::glm4v_invoke_request_method(&mut glm4v_invoke_model, token.clone(), input.clone(), user_config, default_url.clone()).await?;
        let response_message = glm4v_invoke_model.ai_response_data.clone();
        let result = glm4v_invoke_model.process_glm4v_task_status(&*response_message);
        Ok(result)
    }

    async fn generate_glm4v_json_request_body(
        model: &str,
        user_role: String,
        user_input: String,
    ) -> Result<String, Box<dyn Error>> {
        let user_array_message = vec![create_json_message(user_role, user_input)];

        let json_request_body = json!({
        "model": model,
        "messages": user_array_message,
        "stream": true
    });

        let json_string = serde_json::to_string(&json_request_body)?;
        let result = json_string.replace(r"\\\\", r"\\").replace(r"\\", r"").trim().to_string();

        Ok(result)
    }


    pub async fn glm4v_invoke_request_method(
        &mut self,
        token: String,
        user_input: String,
        user_config: &str,
        default_url: String,
    ) -> Result<String, String> {
        let json_string = match glm4v_read_config(user_config, "glm-4v").await {
            Ok(final_json_string) => final_json_string,
            Err(err) => return Err(format!("Error reading config file: {}", err)),
        };

        let glm4v_json_value: Value = serde_json::from_str(&json_string).expect("Failed to parse Toml to JSON");
        let model = glm4v_json_value[0]["model"].as_str().ok_or("Failed to get model")?.to_string();
        let user_role = glm4v_json_value[0]["user_role"].as_str().ok_or("Failed to get user_role")?.to_string();

        let user_json4v_content = match Self::generate_glm4v_json_request_body(
            &model,
            user_role,
            user_input,
        ).await {
            Ok(result) => result.to_string(),
            Err(err) => return Err(err.to_string()),
        };

        let request_result_glm4v = reqwest::Client::new()
            .post(&default_url)
            .header("Cache-Control", "no-cache")
            .header("Connection", "keep-alive")
            .header("Accept", "text/event-stream")
            .header("Content-Type", "application/json;charset=UTF-8")
            .header("Authorization", format!("Bearer {}", token))
            .body(user_json4v_content)
            .send()
            .await
            .map_err(|err| format!("HTTP request failure: {}", err))?;

        if !request_result_glm4v.status().is_success() {
            return Err(format!("Server returned an error: {}", request_result_glm4v.status()).into());
        }

        let mut response_body = request_result_glm4v.bytes_stream();
        let mut sse_glm4v_data = String::new();

        // 处理 SSE-GLM4v 事件
        while let Some(chunk) = response_body.next().await {
            match chunk {
                Ok(bytes) => {
                    let data = String::from_utf8_lossy(&bytes);
                    sse_glm4v_data.push_str(&data);
                    self.ai_response_data = sse_glm4v_data.clone();

                    if data.contains("data: [DONE]") {
                        break;
                    }
                }
                Err(e) => {
                    return Err(format!("Error receiving SSE-glm4v event: {}", e).into());
                }
            }
        }

        Ok(sse_glm4v_data)
    }

    fn process_glm4v_task_status(&mut self, response_data: &str) -> String {
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
}
