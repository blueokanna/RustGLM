mod history_message;

extern crate toml;

use std::fs::File;
use std::collections::VecDeque;
use std::error::Error;
use std::io::Read;
use regex::Regex;
use reqwest;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use futures::stream::StreamExt;

lazy_static::lazy_static! {
    static ref UNICODE_REGEX: regex::Regex = regex::Regex::new(r"\\u[0-9a-fA-F]{4}").unwrap();
}

/*
ChatGLM-3, ChatGLM-4 Config
*/

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
struct SSEConfig {
    ai_config_glm3: Vec<AiResponse>,
    ai_config_glm4: Vec<AiResponse>,
}

fn sse_read_config(file_path: &str, glm: &str) -> Result<String, Box<dyn Error>> {
    let mut file = File::open(file_path)?;
    let mut file_content = String::new();
    file.read_to_string(&mut file_content)?;

    let config: SSEConfig = toml::from_str(&file_content)?;

    let response = match glm {
        "glm-3" => &config.ai_config_glm3,
        "glm-4" => &config.ai_config_glm4,
        _ => return Err("Invalid glm-format".into()),
    };

    serde_json::to_string(response).map_err(Into::into)
}


/*
ChatGLM-4V Config
*/

#[derive(Serialize, Deserialize, Debug)]
struct Glm4vConfig {
    model: Option<String>,
    user_role: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
struct GLM4VConfig {
    ai_config_glm4v: Vec<Glm4vConfig>,
}

async fn glm4v_read_config(file_path: &str, glm: &str) -> Result<String, Box<dyn Error>> {
    let file_content = tokio::fs::read_to_string(file_path).await?;
    let config: GLM4VConfig = toml::from_str(&file_content)?;

    let response = match glm {
        "glm-4v" => config.ai_config_glm4v,
        _ => return Err("Invalid glm4v".into()),
    };

    let json_string = serde_json::to_string(&response)?;

    Ok(json_string)
}


/*
Create chatglm-4v message format by Regex
*/


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

fn create_4vjson_message(user_role: String, user_input: String) -> JSONResonseData {
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

/*
History Message Controller(Save Messages)
*/

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
pub struct SSEInvokeModel {
    get_message: String,
    ai_response_data: String,
}

impl SSEInvokeModel {
    pub fn new() -> Self {
        SSEInvokeModel {
            get_message: String::new(),
            ai_response_data: String::new(),
        }
    }

    pub async fn sse_request(token: String, input: String, glm_version: &str, user_config: &str, default_url: String) -> Result<String, Box<dyn Error>> {
        let mut sse_invoke_model = Self::new();
        Self::sse_invoke_request_method(&mut sse_invoke_model, token.clone(), input.clone(), glm_version, user_config, default_url.clone()).await?;
        let response_message = sse_invoke_model.ai_response_data.clone();
        let result = sse_invoke_model.process_sse_message(&*response_message, &input);
        Ok(result)
    }


    /*
    GLM4V request body by JSON
     */

    async fn generate_glm4v_json_request_body(
        model: &str,
        user_role: String,
        user_input: String,
    ) -> Result<String, Box<dyn Error>> {
        let user_array_message = vec![create_4vjson_message(user_role, user_input)];

        let json_request_body = json!({
        "model": model,
        "messages": user_array_message,
        "stream": true
    });

        let json_string = serde_json::to_string(&json_request_body)?;
        let result = json_string.replace(r"\\\\", r"\\").replace(r"\\", r"").trim().to_string();

        Ok(result)
    }


    /*
    ChatGLM3 / 4 request body by JSON
     */

    async fn generate_sse_json_request_body(
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
        "stream": true,
        "do_sample":true,
        "max_tokens":max_token,
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


    /*
     GLM4V_Handler Request by async
     */

    async fn glm4v_handle_sse_request(user_config: &str, part2_content: String) -> Result<String, Box<dyn Error>> {
        let json_string = match glm4v_read_config(user_config, "glm-4v").await {
            Ok(json_string) => json_string,
            Err(err) => return Err(Box::from(format!("Error reading config file: {}", err))),
        };

        let glm4v_json_value: Value = serde_json::from_str(&json_string)
            .map_err(|err| Box::new(err))?;

        let model = glm4v_json_value[0]["model"].as_str().ok_or("Failed to get model")?.to_string();
        let user_role = glm4v_json_value[0]["user_role"].as_str().ok_or("Failed to get user_role")?.to_string();

        Ok(Self::generate_glm4v_json_request_body(
            &model,
            user_role,
            part2_content,
        ).await?
            .to_string())
    }


    /*
     Normal_Handler Request by async
     */

    async fn async_handle_sse_request(user_config: &str, glm_version: &str, part2_content: String) -> Result<String, Box<dyn Error>> {
        let json_string = match sse_read_config(user_config, glm_version) {
            Ok(json_string) => json_string,
            Err(err) => return Err(Box::from(format!("Error reading config file: {}", err))),
        };

        let json_value: Value = serde_json::from_str(&json_string)
            .map_err(|err| Box::new(err))?;

        let language_model = json_value[0]["language_model"]
            .as_str()
            .ok_or("Failed to get language_model")?
            .to_string();
        let system_role = json_value[0]["system_role"]
            .as_str()
            .ok_or("Failed to get system_role")?
            .to_string();
        let system_content = json_value[0]["system_content"]
            .as_str()
            .ok_or("Failed to get system_content")?
            .trim()
            .to_string();
        let user_role = json_value[0]["user_role"]
            .as_str()
            .ok_or("Failed to get user_role")?
            .to_string();
        let max_token = json_value[0]["max_tokens"]
            .as_f64()
            .ok_or("Failed to get max_token")?;
        let temp_float = json_value[0]["temp_float"]
            .as_f64()
            .ok_or("Failed to get temp_float")?;
        let top_p_float = json_value[0]["top_p_float"]
            .as_f64()
            .ok_or("Failed to get top_p_float")?;

        Ok(Self::generate_sse_json_request_body(
            &language_model,
            &system_role,
            &system_content,
            &user_role,
            &part2_content,
            max_token,
            temp_float,
            top_p_float,
        ).await?
            .to_string())
    }

    async fn async_mode_checker(require_calling: String) -> bool {
        require_calling.to_lowercase() == "glm4v"
    }

    async fn regex_checker(regex: &Regex, input: String) -> bool {
        regex.is_match(&*input)
    }

    async fn json_content_post_function(user_input: String, glm_version: &str, user_config: &str) -> String {
        let regex_in = Regex::new(r"(.*?):(.*)").unwrap();

        if SSEInvokeModel::regex_checker(&regex_in, user_input.clone()).await {
            let mut part1_content = String::new();
            let mut part2_content = String::new();

            if let Some(captures_message) = regex_in.captures(&user_input) {
                if let Some(first_part) = captures_message.get(1) {
                    part1_content = first_part.as_str().to_string();
                }
                if let Some(second_part) = captures_message.get(2) {
                    part2_content = second_part.as_str().to_string();
                }
            } else {
                println!("Input does not match the pattern");
                return String::new();
            }

            if SSEInvokeModel::async_mode_checker(part1_content.clone()).await {
                match SSEInvokeModel::glm4v_handle_sse_request(user_config, part2_content.clone()).await {
                    Ok(result) => result,
                    Err(err) => {
                        println!("{}", err);
                        return String::new();
                    }
                }
            } else {
                match SSEInvokeModel::async_handle_sse_request(user_config, glm_version, user_input.clone()).await {
                    Ok(result) => result,
                    Err(err) => {
                        println!("{}", err);
                        return String::new();
                    }
                }
            }
        } else {
            match SSEInvokeModel::async_handle_sse_request(user_config, glm_version, user_input.clone()).await {
                Ok(result) => result,
                Err(err) => {
                    println!("{}", err);
                    return String::new();
                }
            }
        }
    }

    pub async fn sse_invoke_request_method(
        &mut self,
        token: String,
        user_input: String,
        glm_version: &str,
        user_config: &str,
        default_url: String,
    ) -> Result<String, String> {
        /*
                println!("LANGUAGE_MODEL: {}", language_model);
                println!("SYSTEM_ROLE: {}", system_role);
                println!("SYSTEM_CONTENT: {}", system_content);
                println!("USER_ROLE: {}", user_role);
                println!("Token_NUM: {}", max_token);
                println!("TEMP_FLOAT: {}", temp_float);
                println!("TOP_P_FLOAT: {}", top_p_float);
        */
        let post_json = Self::json_content_post_function(user_input, glm_version, user_config);

        let request_result = reqwest::Client::new()
            .post(&default_url)
            .header("Cache-Control", "no-cache")
            .header("Connection", "keep-alive")
            .header("Accept", "text/event-stream")
            .header("Content-Type", "application/json;charset=UTF-8")
            .header("Authorization", format!("Bearer {}", token))
            .body(post_json.await)
            .send()
            .await
            .map_err(|err| format!("HTTP request failure: {}", err))?;

        if !request_result.status().is_success() {
            return Err(format!("Server returned an error: {}", request_result.status()));
        }

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
            message_process.add_history_to_file("user", user_message);
            message_process.add_history_to_file("assistant", &*queue_result);
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
}
