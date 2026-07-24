use rustglm::{BatchCreateRequest, ZhipuClient};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let input_file_id = std::env::args()
        .nth(1)
        .ok_or("usage: cargo run --example batch_management -- <input-file-id>")?;
    let client = ZhipuClient::new(std::env::var("ZHIPU_API_KEY")?)?;
    let created = client
        .create_batch(&BatchCreateRequest::new(
            input_file_id,
            "/v4/chat/completions",
        ))
        .await?;
    println!("created={} status={:?}", created.id, created.status);

    let listed = client.batches(Some(20), None).await?;
    println!("listed={}", listed.data.len());
    let current = client.batch(&created.id).await?;
    println!("current={:?}", current.status);
    let cancelled = client.cancel_batch(&created.id).await?;
    println!("cancelled={:?}", cancelled.status);
    Ok(())
}
