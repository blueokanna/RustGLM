use rustglm::{VideoGenerationRequest, ZhipuClient};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let prompt = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "A slow camera move through a modern library".to_owned());
    let client = ZhipuClient::new(std::env::var("ZHIPU_API_KEY")?)?;
    let request = VideoGenerationRequest::new("cogvideox-3")
        .prompt(prompt)
        .quality("quality")
        .size("1920x1080")
        .duration(5)
        .with_audio(true);
    let task = client.create_video(&request).await?;
    println!("task={} status={}", task.id, task.task_status);
    Ok(())
}
