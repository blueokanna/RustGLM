use futures_util::StreamExt;
use rustglm::{ChatCompletionRequest, ChatMessage, ResponseContent, ZhipuClient};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let api_key = std::env::var("ZHIPU_API_KEY")?;
    let client = ZhipuClient::new(api_key)?;
    let request = ChatCompletionRequest::new("glm-5.3")
        .message(ChatMessage::system("Answer concisely."))
        .message(ChatMessage::user(
            "Explain Rust ownership in one paragraph.",
        ));
    let mut stream = client.chat_completion_stream(&request).await?;

    while let Some(chunk) = stream.next().await {
        for choice in chunk?.choices {
            if let Some(ResponseContent::Text(text)) = choice.delta.content {
                print!("{text}");
            }
        }
    }
    println!();
    Ok(())
}
