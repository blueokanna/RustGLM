use nextjson::json;
use rustglm::ZhipuClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let api_key = std::env::var("ZHIPU_API_KEY")?;
    let query = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "Rust async streams".to_owned());
    let client = ZhipuClient::new(api_key)?;
    let response = client
        .web_search(&json!({
            "search_query": query,
            "search_engine": "search_std",
        }))
        .await?;
    println!("{response:#}");
    Ok(())
}
