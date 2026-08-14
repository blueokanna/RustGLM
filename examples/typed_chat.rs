use rustglm::{Glm53, ReasoningEffort, Thinking, TypedChatRequest, ZhipuClient};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = ZhipuClient::new(std::env::var("ZHIPU_API_KEY")?)?;
    let request = TypedChatRequest::<Glm53>::new()
        .system("Show the important reasoning, then give the answer.")
        .thinking(Thinking::enabled())
        .reasoning_effort(ReasoningEffort::High)
        .user("Compare optimistic and pessimistic concurrency control.");
    let response = client.typed_chat_completion(&request).await?;
    println!("{}", response.text().unwrap_or_default());
    Ok(())
}
