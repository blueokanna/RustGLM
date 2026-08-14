use rustglm::ZhipuClient;
use nextjson::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let image_url = std::env::args()
        .nth(1)
        .ok_or("usage: cargo run --example document_understanding -- <image-url>")?;
    let client = ZhipuClient::new(std::env::var("ZHIPU_API_KEY")?)?;
    let ocr = client
        .ocr(&json!({
            "model": "glm-ocr",
            "file_url": image_url,
            "tool_type": "hand_write",
            "language_type": "CHN_ENG"
        }))
        .await?;
    println!("ocr: {ocr:#}");

    let layout = client.parse_layout(&json!({"file_url": image_url})).await?;
    println!("layout: {layout:#}");
    Ok(())
}
