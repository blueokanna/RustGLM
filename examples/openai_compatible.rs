use rustglm::{ChatCompletionRequest, ChatMessage, ChatProvider, OpenAiCompatibleConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let base_url = std::env::var("OPENAI_COMPATIBLE_BASE_URL")?;
    let api_key = std::env::var("OPENAI_COMPATIBLE_API_KEY")?;
    let client = OpenAiCompatibleConfig::new("compatible", api_key, base_url).build()?;
    let request = ChatCompletionRequest::new("model-name")
        .message(ChatMessage::user("Reply with the word ready."));
    let response = ChatProvider::complete(&client, request).await?;
    println!("{}", response.text().unwrap_or_default());
    Ok(())
}
