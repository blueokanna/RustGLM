use futures_util::StreamExt;
use rustglm::{
    FunctionDefinition, Glm53, ResponseContent, Tool, ToolStreamEvent, TypedChatRequest,
    ZhipuClient,
};
use nextjson::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let tool = Tool::function(
        FunctionDefinition::new(
            "deployment_status",
            json!({
                "type": "object",
                "properties": {"service": {"type": "string"}},
                "required": ["service"]
            }),
        )
        .description("Read the current deployment status"),
    );
    let request = TypedChatRequest::<Glm53>::new()
        .tool(tool)
        .tool_stream()
        .user("Check the deployment status of payments-api.");
    let client = ZhipuClient::new(std::env::var("ZHIPU_API_KEY")?)?;
    let mut stream = client.typed_chat_tool_stream(&request).await?;
    while let Some(event) = stream.next().await {
        match event? {
            ToolStreamEvent::ContentDelta {
                content: ResponseContent::Text(text),
                ..
            } => print!("{text}"),
            ToolStreamEvent::ToolCallCompleted(call) => {
                println!("tool={} arguments={}", call.name, call.arguments);
            }
            _ => {}
        }
    }
    Ok(())
}
