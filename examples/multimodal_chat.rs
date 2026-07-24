use rustglm::{ChatCompletionRequest, ChatMessage, ContentPart, MessageRole, ZhipuClient};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let api_key = std::env::var("ZHIPU_API_KEY")?;
    let image_url = std::env::var("IMAGE_URL")?;
    let client = ZhipuClient::new(api_key)?;
    let request = ChatCompletionRequest::new("glm-5v-turbo").message(ChatMessage::multimodal(
        MessageRole::User,
        vec![
            ContentPart::image_url(image_url),
            ContentPart::text("Describe this image in one concise paragraph."),
        ],
    ));
    let response = client.chat_completion(&request).await?;
    println!("{}", response.text().unwrap_or_default());
    Ok(())
}
