use rustglm::{EmbeddingRequest, ZhipuClient};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = ZhipuClient::new(std::env::var("ZHIPU_API_KEY")?)?;
    let request = EmbeddingRequest::new(
        "embedding-3",
        vec!["Rust ownership".to_owned(), "Borrow checking".to_owned()],
    )
    .dimensions(1024);
    let response = client.embedding(&request).await?;
    for item in response.data {
        println!("index={} dimensions={}", item.index, item.embedding.len());
    }
    Ok(())
}
