mod custom_jwt;
mod api_operation;
mod async_invoke_method;
mod sync_invoke_method;
mod sse_invoke_method;

use std::error::Error;
use std::io;
use tokio::io::AsyncWriteExt;

#[derive(Debug)]
pub struct RustGLM {
    chatglm_response: String,
}

impl RustGLM {
    pub async fn new() -> Self {
        RustGLM {
            chatglm_response: String::new(),
        }
    }

    async fn async_invoke_calling(jwt_token: &str, user_input: &str) -> String {
        let jwt_token_clone = jwt_token.to_string();
        let user_input_clone = user_input.to_string();

        let handle = tokio::spawn(async move {
            let response =
                async_invoke_method::ReceiveAsyncInvokeOnlyText::new(&jwt_token_clone, &user_input_clone);
            response
                .await
                .get_response()
                .map(|s| s.to_string())
                .unwrap_or_else(|| "Error getting response.".to_string())
        });

        handle.await.expect("Failed to await JoinHandle")
    }

    async fn sync_invoke_calling(jwt_token: &str, user_input: &str) -> String {
        let sync_call = sync_invoke_method::ReceiveInvokeModelOnlyText::new(jwt_token, user_input);

        match sync_call.await.get_response_message() {
            Some(message) => message.to_string(), // Return the message as String
            None => "Error: Unable to get sync response.".to_string(),
        }
    }

    async fn sse_invoke_calling(jwt_token: &str, user_input: &str) -> String {
        let sse_call = sse_invoke_method::ReceiveSSEInvokeModelOnlyText::new(jwt_token, user_input);

        match sse_call.await.get_response_message() {
            Some(message) => message.to_string(), // Return the message as String
            None => "Error: Unable to get SSE response.".to_string(),
        }
    }

    pub async fn rust_chat_glm(&mut self) -> String {
        let mut api_key = api_operation::APIKeys::load_api_key();
        let mut require_calling = "SSE".to_string();
        let mut ai_message = String::new();
        let mut input = String::new();

        if api_key.is_none() {
            println!("Enter your API Key:");
            if let Ok(_) = io::stdin().read_line(&mut input) {
                api_key = Some(input.trim().to_string());
                api_operation::APIKeys::save_api_key(api_key.as_ref().unwrap());
            } else {
                eprintln!("Unable to read user input");
                return String::new();
            }
        }

        if let Some(api_key) = api_key {
            let api_key_instance = api_operation::APIKeys::get_instance(&*api_key);
            let jwt_creator =
                custom_jwt::CustomJwt::new(api_key_instance.get_user_id(), api_key_instance.get_user_secret());
            let jwt = jwt_creator.create_jwt();

            let jwt_to_verify = jwt.clone();
            let is_valid = jwt_creator.verify_jwt(&jwt_to_verify);

            if is_valid {
                loop {
                    //println!("You:");
                    let mut user_input = String::new();
                    io::stdin().read_line(&mut user_input).expect("Failed to read input");

                    match user_input.trim().to_lowercase().as_str() {
                        "sse" => {
                            require_calling = "SSE".to_string();
                            println!("Calling method is SSE");
                            continue;
                        }
                        "async" => {
                            require_calling = "ASYNC".to_string();
                            println!("Calling method is Async");
                            continue;
                        }
                        "sync" => {
                            require_calling = "SYNC".to_string();
                            println!("Calling method is Sync");
                            continue;
                        }
                        "exit" => {
                            break; // Exit the loop if "exit" is entered
                        }
                        _ => {}
                    }

                    if require_calling == "SSE" || require_calling == "sse" {
                        ai_message = Self::sse_invoke_calling(&jwt, &user_input.trim()).await;
                    } else if require_calling == "async" || require_calling == "ASYNC" || require_calling == "Async" {
                        ai_message = Self::async_invoke_calling(&jwt, &user_input.trim()).await;
                    } else if require_calling == "sync" || require_calling == "SYNC" || require_calling == "Sync" {
                        ai_message = Self::sync_invoke_calling(&jwt, &user_input.trim()).await;
                    }

                    self.chatglm_response = ai_message.clone();
                    return ai_message;
                }
            } else {
                println!("JWT is NOT valid");
            }
        } else {
            println!("API Key not found or an error occurred while loading.");
        }

        String::new()
    }
    pub fn get_ai_response(&self) -> String {
        self.chatglm_response.clone()
    }
}