use std::path::Path;

use rustglm::{FileUploadRequest, ZhipuClient};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let input = std::env::args()
        .nth(1)
        .ok_or("usage: cargo run --example file_management -- path/to/file")?;
    let path = Path::new(&input);
    let file_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or("path must have a UTF-8 file name")?;
    let client = ZhipuClient::new(std::env::var("ZHIPU_API_KEY")?)?;
    let uploaded = client
        .upload_file(FileUploadRequest::from_bytes(
            file_name,
            std::fs::read(path)?,
            "file-extract",
        ))
        .await?;
    println!("uploaded={}", uploaded.id);

    let files = client.files(Some("file-extract"), Some(20)).await?;
    println!("listed={}", files.data.len());
    let content = client.file_content(&uploaded.id).await?;
    println!("downloaded={} bytes", content.len());
    let deleted = client.delete_file(&uploaded.id).await?;
    println!("deleted={}", deleted.deleted);
    Ok(())
}
