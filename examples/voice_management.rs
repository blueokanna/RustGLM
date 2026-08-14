use nextjson::json;
use rustglm::ZhipuClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let operation = args.next().unwrap_or_else(|| "list".to_owned());
    let client = ZhipuClient::new(std::env::var("ZHIPU_API_KEY")?)?;
    let response = match operation.as_str() {
        "clone" => {
            let file_id = args
                .next()
                .ok_or("voice_management clone <file-id> <name>")?;
            let voice_name = args
                .next()
                .ok_or("voice_management clone <file-id> <name>")?;
            client
                .clone_voice(&json!({
                    "model": "glm-tts-clone",
                    "voice_name": voice_name,
                    "input": "你好，这是试听语音。",
                    "file_id": file_id
                }))
                .await?
        }
        "delete" => {
            let voice = args.next().ok_or("voice_management delete <voice-id>")?;
            client.delete_voice(&json!({"voice": voice})).await?
        }
        "list" => client.voices().await?,
        _ => return Err("operation must be list, clone, or delete".into()),
    };
    println!("{response:#}");
    Ok(())
}
