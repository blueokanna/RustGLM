use std::path::Path;

use rustglm::{TranscriptionRequest, ZhipuClient};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let input = std::env::args()
        .nth(1)
        .ok_or("usage: cargo run --example transcription -- path/to/audio.wav")?;
    let path = Path::new(&input);
    let file_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or("audio path must have a UTF-8 file name")?;
    let request = TranscriptionRequest::from_bytes("glm-asr-2512", file_name, std::fs::read(path)?)
        .prompt("Produce a verbatim transcript")
        .hotwords(["RustGLM", "GLM"]);
    let client = ZhipuClient::new(std::env::var("ZHIPU_API_KEY")?)?;
    let response = client.transcribe(request).await?;
    println!("{}", response.text);
    Ok(())
}
