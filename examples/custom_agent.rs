use std::io::{self, Write};
use std::sync::Arc;

use async_trait::async_trait;
use rustglm::{
    AgentHistoryPolicy, AgentManifest, AgentPersona, AgentRuntime, AgentTool, FunctionDefinition,
    Result as SdkResult, ZhipuClient,
};
use nextjson::{Value, json};

struct DeviceInfoTool;

#[async_trait]
impl AgentTool for DeviceInfoTool {
    fn definition(&self) -> FunctionDefinition {
        FunctionDefinition::new(
            "device_info",
            json!({"type":"object","properties":{},"additionalProperties":false}),
        )
        .description("Return the operating system and CPU architecture running this application")
    }

    async fn execute(&self, _: Value) -> SdkResult<Value> {
        Ok(json!({"os":std::env::consts::OS,"arch":std::env::consts::ARCH}))
    }
}

fn read_line(prompt: &str) -> io::Result<String> {
    print!("{prompt}");
    io::stdout().flush()?;
    let mut value = String::new();
    io::stdin().read_line(&mut value)?;
    Ok(value.trim().to_owned())
}

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let api_key = match std::env::var("ZHIPU_API_KEY") {
        Ok(value) if !value.trim().is_empty() => value,
        _ => read_line("请输入智谱 API Key（输入内容会显示）: ")?,
    };
    let model = match read_line("模型名称，直接回车使用 glm-4-flash: ")? {
        value if value.is_empty() => "glm-4-flash".to_owned(),
        value => value,
    };
    let persona = AgentPersona::new("洛书", "跨平台 Rust 技术伙伴")
        .background("熟悉桌面、服务器和移动设备上的 Rust 应用部署")
        .trait_value("准确")
        .trait_value("有明确观点")
        .speaking_style("简洁、自然、先给结论")
        .language("简体中文")
        .instruction("需要设备信息时调用 device_info")
        .boundary("不要假装执行未注册的工具");
    let manifest =
        AgentManifest::new(model, persona).history(AgentHistoryPolicy::Recent { max_messages: 20 });
    println!("可部署清单不包含密钥:\n{}", manifest.to_json()?);

    let client = ZhipuClient::new(api_key)?;
    let mut agent = AgentRuntime::new(Arc::new(client), manifest)?;
    agent.register_tool(DeviceInfoTool)?;
    let question = match read_line("请输入问题，直接回车询问当前设备: ")? {
        value if value.is_empty() => "当前程序运行在什么设备架构上？".to_owned(),
        value => value,
    };
    let result = agent.run(question).await?;
    println!("{}", result.response.text().unwrap_or_default());
    println!("模型调用步数: {}", result.model_steps);
    println!("工具调用次数: {}", result.tool_executions.len());
    Ok(())
}
