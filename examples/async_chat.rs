use rustglm::{ChatCompletionRequest, ChatMessage, ZhipuClient};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = ZhipuClient::new(std::env::var("ZHIPU_API_KEY")?)?;
    let request = ChatCompletionRequest::new("glm-5.3")
        .message(ChatMessage::user("Write a detailed migration checklist."));
    let task = client.async_chat(&request).await?;
    println!("submitted task={} status={}", task.id, task.task_status);

    let result = client.async_result(&task.id).await?;
    println!("task={} status={}", result.id, result.task_status);
    Ok(())
}
