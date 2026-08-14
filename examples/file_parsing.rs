use rustglm::ZhipuClient;
use nextjson::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let file_url = std::env::args()
        .nth(1)
        .ok_or("usage: cargo run --example file_parsing -- https://example.com/document.pdf")?;
    let client = ZhipuClient::new(std::env::var("ZHIPU_API_KEY")?)?;

    let task = client
        .create_file_parse_task(&json!({"file_url": file_url, "tool_type": "lite"}))
        .await?;
    if let Some(task_id) = task.get("task_id").and_then(|value| value.as_str()) {
        let result = client.file_parse_result(task_id, "text").await?;
        println!("async parse: {result:#}");
    } else {
        println!("parse task: {task:#}");
    }

    let parsed = client
        .parse_file_sync(&json!({"file_url": file_url, "tool_type": "lite"}))
        .await?;
    println!("sync parse: {parsed:#}");
    Ok(())
}
