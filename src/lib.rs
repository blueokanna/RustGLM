mod custom_jwt;
mod api_operation;
mod async_invoke_method;
mod sync_invoke_method;
mod sse_invoke_method;
mod cogview_invoke_method;
mod glm4v_invoke_method;

use std::io;

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

    async fn async_invoke_calling(jwt_token: &str, user_input: &str, user_config: &str) -> String {
        let jwt_token_clone = jwt_token.to_string();
        let user_input_clone = user_input.to_string();
        let user_config_clone = user_config.to_string();

        let handle = tokio::spawn(async move {
            let response =
                async_invoke_method::ReceiveAsyncInvokeOnlyText::new(&jwt_token_clone, &user_input_clone, user_config_clone);
            response
                .await
                .get_response()
                .map(|s| s.to_string())
                .unwrap_or_else(|| "Error getting response.".to_string())
        });

        handle.await.expect("Failed to await JoinHandle")
    }

    async fn sync_invoke_calling(jwt_token: &str, user_input: &str, user_config: &str) -> String {
        let sync_call = sync_invoke_method::ReceiveInvokeModelOnlyText::new(jwt_token, user_input, user_config);

        match sync_call.await.get_response_message() {
            Some(message) => message.to_string(), // Return the message as String
            None => "Error: Unable to get sync response.".to_string(),
        }
    }

    async fn sse_invoke_calling(jwt_token: &str, user_input: &str, user_config: &str) -> String {
        let sse_call = sse_invoke_method::ReceiveSSEInvokeModelOnlyText::new(jwt_token, user_input, user_config);

        match sse_call.await.get_response_message() {
            Some(message) => message.to_string(), // Return the message as String
            None => "Error: Unable to get SSE response.".to_string(),
        }
    }

    async fn cogview_invoke_calling(jwt_token: &str, user_input: &str, user_config: &str) -> String {
        let cogview_sync_call = cogview_invoke_method::ReceiveCogviewInvokeModel::new(jwt_token, user_input, user_config);

        match cogview_sync_call.await.get_cogview_response_message() {
            Some(message) => message.to_string(),
            None => "Error: Unable to get CogView response.".to_string(),
        }
    }

    async fn glm4v_invoke_calling(jwt_token: &str, user_input: &str, user_config: &str) -> String {
        let glm4v_sse_call = glm4v_invoke_method::Receive4VInvokeModelwithText::new(jwt_token, user_input, user_config);

        match glm4v_sse_call.await.get_response_glm4v_message() {
            Some(message) => message.to_string(),
            None => "Error: Unable to get glm4v response.".to_string(),
        }
    }

    pub async fn rust_chat_glm(&mut self, user_config: &str) -> String {
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
                        "cogview" => {
                            require_calling = "COGVIEW".to_string();
                            println!("Calling method is CogView");
                            continue;
                        }

                        "glm4v" => {
                            require_calling = "GLM4V".to_string();
                            println!("Calling method is glm4v");
                            continue;
                        }

                        "exit" => {
                            break; // Exit the loop if "exit" is entered
                        }
                        _ => {}
                    }

                    if require_calling == "SSE" || require_calling == "sse" {
                        ai_message = Self::sse_invoke_calling(&jwt, &user_input.trim(), user_config).await;
                    } else if require_calling == "async" || require_calling == "ASYNC" || require_calling == "Async" {
                        ai_message = Self::async_invoke_calling(&jwt, &user_input.trim(), user_config).await;
                    } else if require_calling == "sync" || require_calling == "SYNC" || require_calling == "Sync" {
                        ai_message = Self::sync_invoke_calling(&jwt, &user_input.trim(), user_config).await;
                    } else if require_calling == "cogview" || require_calling == "COGVIEW" || require_calling == "CogView" || require_calling == "Cogview" {
                        ai_message = Self::cogview_invoke_calling(&jwt, &user_input.trim(), user_config).await;
                    } else if require_calling == "glm4v" || require_calling == "GLM4V" || require_calling == "GLM4v" || require_calling == "glm4V" {
                        ai_message = Self::glm4v_invoke_calling(&jwt, &user_input.trim(), user_config).await;
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
