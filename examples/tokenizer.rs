use rustglm::{ChatMessage, TokenizerRequest, ZhipuClient};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = ZhipuClient::new(std::env::var("ZHIPU_API_KEY")?)?;
    let request = TokenizerRequest::new(
        "glm-5.3",
        [ChatMessage::user("Count the tokens in this message.")],
    );
    let response = client.tokenizer(&request).await?;
    println!("total_tokens={}", response.usage.total_tokens);
    Ok(())
}
