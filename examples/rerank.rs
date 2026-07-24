use rustglm::{RerankRequest, ZhipuClient};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = ZhipuClient::new(std::env::var("ZHIPU_API_KEY")?)?;
    let request = RerankRequest::new(
        "rerank",
        "How does Rust manage memory?",
        [
            "Rust uses ownership and borrowing.",
            "Garbage collection runs periodically.",
            "The moon orbits Earth.",
        ],
    )
    .top_n(2)
    .return_documents(true);
    let response = client.rerank(&request).await?;
    for result in response.results {
        println!("index={} score={}", result.index, result.relevance_score);
    }
    Ok(())
}
