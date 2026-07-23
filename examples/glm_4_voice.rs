use std::fs;
use std::io::{self, Write};

use rustglm::{Glm4VoiceRequest, ZhipuClient};

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
    let input_path = read_line("请输入 WAV 输入文件路径: ")?;
    let prompt = match read_line("语音指令，直接回车使用 慢速复述这段语音: ")? {
        value if value.is_empty() => "慢速复述这段语音".to_owned(),
        value => value,
    };
    let output_path = match read_line("输出文件，直接回车使用 glm-4-voice-output.wav: ")?
    {
        value if value.is_empty() => "glm-4-voice-output.wav".to_owned(),
        value => value,
    };
    let wav = fs::read(&input_path)?;
    if wav.len() < 12 || &wav[..4] != b"RIFF" || &wav[8..12] != b"WAVE" {
        return Err("输入文件不是有效的 RIFF/WAVE 文件".into());
    }
    let client = ZhipuClient::new(api_key)?;
    let response = client
        .glm_4_voice(&Glm4VoiceRequest::from_wav(prompt, &wav)?)
        .await?;
    println!("文本响应: {}", response.text().unwrap_or_default());
    let output = response.audio_wav()?.ok_or("响应中没有音频数据")?;
    fs::write(&output_path, output)?;
    println!("已保存 44.1kHz 单声道 PCM16 WAV: {output_path}");
    Ok(())
}
