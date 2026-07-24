use rustglm::{SpeechRequest, ZhipuClient};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let input = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "你好，这是一段由 RustGLM 合成的语音。".to_owned());
    let client = ZhipuClient::new(std::env::var("ZHIPU_API_KEY")?)?;
    let request = SpeechRequest::new("glm-tts", input, "tongtong")
        .speed(1.0)
        .volume(1.0)
        .response_format("wav");
    let audio = client.speech(&request).await?;
    println!("received {} audio bytes", audio.len());
    Ok(())
}
