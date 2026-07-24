use rustglm::{ImageGenerationRequest, ZhipuClient};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    let asynchronous = args.iter().any(|argument| argument == "--async");
    let prompt = args
        .into_iter()
        .find(|argument| argument != "--async")
        .unwrap_or_else(|| "A precise technical cutaway of a mechanical watch".to_owned());
    let client = ZhipuClient::new(std::env::var("ZHIPU_API_KEY")?)?;
    let request = ImageGenerationRequest::new("cogview-4-250304", prompt)
        .size("1024x1024")
        .quality("hd")
        .watermark(false);
    if asynchronous {
        let task = client.create_image_async(&request).await?;
        println!("task={} status={}", task.id, task.task_status);
        return Ok(());
    }
    let response = client.create_image(&request).await?;
    for image in response.data {
        println!("{}", image.url.as_deref().unwrap_or("inline base64 image"));
    }
    Ok(())
}
