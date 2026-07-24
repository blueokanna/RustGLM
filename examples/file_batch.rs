use std::path::Path;

use rustglm::{BatchCreateRequest, FileUploadRequest, ZhipuClient};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let api_key = std::env::var("ZHIPU_API_KEY")?;
    let input_path = std::env::args()
        .nth(1)
        .ok_or("usage: cargo run --example file_batch -- path/to/input.jsonl")?;
    let path = Path::new(&input_path);
    let file_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or("input path must have a UTF-8 file name")?;
    let client = ZhipuClient::new(api_key)?;
    let uploaded = client
        .upload_file(FileUploadRequest {
            file_name: file_name.to_owned(),
            file: std::fs::read(path)?,
            mime_type: Some("application/jsonl".into()),
            purpose: "batch".into(),
        })
        .await?;
    let batch = client
        .create_batch(&BatchCreateRequest::new(
            uploaded.id,
            "/v4/chat/completions",
        ))
        .await?;

    println!("batch_id={}", batch.id);
    Ok(())
}
