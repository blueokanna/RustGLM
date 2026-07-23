use std::io::{self, Write};

use rustglm::{OfficialAgentMessage, OfficialAgentRequest, ZhipuClient};

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
    let agent_id = match read_line("智能体 ID，直接回车使用 general_translation: ")? {
        value if value.is_empty() => "general_translation".to_owned(),
        value => value,
    };
    let input = read_line("请输入发送给智能体的内容: ")?;
    let client = ZhipuClient::new(api_key)?;
    let response = client
        .official_agent(
            &OfficialAgentRequest::new(agent_id).message(OfficialAgentMessage::user(input)),
        )
        .await?;
    println!("{response:#?}");
    Ok(())
}
