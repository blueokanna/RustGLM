use std::fs;
use std::io::{self, Write};

use rustglm::{RealtimeConfig, RealtimeSession};

fn read_line(prompt: &str) -> io::Result<String> {
    print!("{prompt}");
    io::stdout().flush()?;
    let mut value = String::new();
    io::stdin().read_line(&mut value)?;
    Ok(value.trim().to_owned())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let api_key = match std::env::var("ZHIPU_API_KEY") {
        Ok(value) if !value.trim().is_empty() => value,
        _ => read_line("请输入智谱 API Key（输入内容会显示）: ")?,
    };
    let audio_path = read_line("请输入 PCM16 16kHz 单声道或 WAV 音频路径: ")?;
    let video_path = read_line("可选：输入 JPEG 视频帧路径，直接回车表示纯音频: ")?;
    let selected_model = read_line("请输入模型名称，直接回车使用 glm-realtime-flash: ")?;
    let model = if selected_model.is_empty() {
        "glm-realtime-flash".to_owned()
    } else {
        selected_model
    };
    let audio = fs::read(&audio_path)?;
    if audio.is_empty() {
        return Err("音频文件不能为空".into());
    }
    let video = if video_path.is_empty() {
        None
    } else {
        Some(fs::read(video_path)?)
    };

    let mut connection = RealtimeConfig::new(api_key).connect().await?;
    while let Some(event) = connection.next_event().await {
        if event?.event_type == "session.created" {
            break;
        }
    }
    let mut session = RealtimeSession::default()
        .model(model)
        .instructions("请用简洁自然的中文回答")
        .input_audio_format(if audio_path.to_ascii_lowercase().ends_with(".wav") {
            "wav"
        } else {
            "pcm16"
        });
    if video.is_some() {
        session = session.video();
    }
    let sender = connection.sender();
    sender.update_session(session).await?;
    for chunk in audio.chunks(3200) {
        sender.append_audio(chunk).await?;
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
    if let Some(frame) = video {
        sender.append_video_frame(&frame).await?;
    }
    sender.commit().await?;
    sender.create_response().await?;

    let mut output_audio = Vec::new();
    while let Some(event) = connection.next_event().await {
        let event = event?;
        if let Some(text) = event.delta_text() {
            print!("{text}");
            io::stdout().flush()?;
        }
        if let Some(bytes) = event.audio_bytes()? {
            output_audio.extend(bytes);
        }
        if let Some(error) = event.error() {
            return Err(format!("Realtime 服务错误: {error}").into());
        }
        if event.event_type == "response.done" {
            break;
        }
    }
    println!();
    if !output_audio.is_empty() {
        fs::write("realtime-output.pcm", output_audio)?;
        println!("模型音频已保存为 realtime-output.pcm（24kHz、单声道、16位 PCM）");
    }
    connection.close().await?;
    Ok(())
}
