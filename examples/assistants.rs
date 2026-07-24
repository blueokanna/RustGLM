use rustglm::ZhipuClient;
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let prompt = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "Explain this API in one paragraph.".to_owned());
    let client = ZhipuClient::new(std::env::var("ZHIPU_API_KEY")?)?;
    let response = client
        .assistant(&json!({
            "assistant_id": "659e54b1b8006379b4b2abd6",
            "messages": [{"role": "user", "content": prompt}],
            "stream": false
        }))
        .await?;
    println!("invoke: {response:#}");
    println!("list: {:#}", client.assistants(&json!({})).await?);
    println!(
        "conversations: {:#}",
        client
            .assistant_conversations(&json!({"assistant_id": "659e54b1b8006379b4b2abd6"}))
            .await?
    );
    Ok(())
}
