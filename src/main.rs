mod custom_jwt;
mod api_operation;
mod async_invoke_method;
mod sync_invoke_method;
mod sse_invoke_method;

use std::error::Error;
use std::io::{self, Write};
use tokio::io::AsyncWriteExt;

pub async fn async_invoke_calling(jwt_token: &str, user_input: &str) -> String {
    let jwt_token_clone = jwt_token.to_string();
    let user_input_clone = user_input.to_string();

    let handle = tokio::spawn(async move {
        let response = async_invoke_method::ReceiveAsyncInvokeOnlyText::new(&jwt_token_clone, &user_input_clone);
        response
            .await
            .get_response()
            .map(|s| s.to_string())
            .unwrap_or_else(|| "Error getting response.".to_string())
    });

    handle.await.expect("Failed to await JoinHandle")
}

pub async fn sync_invoke_calling(jwt_token: &str, user_input: &str) -> String {
    let sync_call = sync_invoke_method::ReceiveInvokeModelOnlyText::new(jwt_token, user_input);

    match sync_call.await.get_response_message() {
        Some(message) => message.to_string(), // Return the message as String
        None => "Error: Unable to get sync response.".to_string(),
    }
}

pub async fn sse_invoke_calling(jwt_token: &str, user_input: &str) -> String {
    let sse_call = sse_invoke_method::ReceiveSSEInvokeModelOnlyText::new(jwt_token, user_input);

    match sse_call.await.get_response_message() {
        Some(message) => message.to_string(), // Return the message as String
        None => "Error: Unable to get SSE response.".to_string(),
    }
}


#[tokio::main]
pub async fn main() {
    let mut api_key = api_operation::APIKeys::load_api_key();
    let mut input = String::new();

    if api_key.is_none() {
        println!("请输入你的 API 密钥:");
        if let Ok(_) = io::stdin().read_line(&mut input) {
            api_key = Some(input.trim().to_string());
            api_operation::APIKeys::save_api_key(api_key.as_ref().unwrap());
        } else {
            eprintln!("无法读取用户输入。");
            return;
        }
    }

    if let Some(api_key) = api_key {
        //println!("Loaded API Key: {}", api_key);

    let api_key_instance = api_operation::APIKeys::get_instance(&*api_key);

    let jwt_creator = custom_jwt::CustomJwt::new(api_key_instance.get_user_id(), api_key_instance.get_user_secret());
    let jwt = jwt_creator.create_jwt();

    let jwt_to_verify = jwt.clone();
    let is_valid = jwt_creator.verify_jwt(&jwt_to_verify);

    if is_valid {
        println!("请输入对话:");

        loop {
            let mut user_input = String::new();
            io::stdin().read_line(&mut user_input).expect("读取输入失败");

            //let ai_message = async_invoke_calling(&jwt, &user_input.trim()).await;
            //let ai_message = sync_invoke_calling(&jwt, &user_input.trim()).await;
            let ai_message = sse_invoke_calling(&jwt, &user_input.trim()).await;
            println!("莉莉娅: {}", ai_message);
            println!("\n你: ");
        }
    } else {
        println!("JWT is NOT valid");
    }

    } else {
        println!("API Key not found or an error occurred while loading.");
    }
}
