use rustglm::ZhipuClient;
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let url = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "https://www.rust-lang.org/".to_owned());
    let client = ZhipuClient::new(std::env::var("ZHIPU_API_KEY")?)?;
    let page = client
        .read_web_page(&json!({"url": url, "return_type": "text"}))
        .await?;
    println!("reader: {page:#}");

    let moderation = client
        .moderate(&json!({"model": "moderation", "input": "A harmless test message"}))
        .await?;
    println!("moderation: {moderation:#}");
    Ok(())
}
