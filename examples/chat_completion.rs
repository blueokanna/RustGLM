use rustglm::{ChatCompletionRequest, ChatMessage, ZhipuClient};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = ZhipuClient::new(std::env::var("ZHIPU_API_KEY")?)?;
    let request = ChatCompletionRequest::new("glm-5.2")
        .message(ChatMessage::system("Answer accurately and concisely."))
        .message(ChatMessage::user("Why does Rust prevent data races?"));
    let response = client.chat_completion(&request).await?;
    println!("{}", response.text().unwrap_or_default());
    Ok(())
}
