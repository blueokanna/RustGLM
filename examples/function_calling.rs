use nextjson::json;
use rustglm::{ChatCompletionRequest, ChatMessage, FunctionDefinition, Tool, ZhipuClient};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = ZhipuClient::new(std::env::var("ZHIPU_API_KEY")?)?;
    let weather = FunctionDefinition::new(
        "current_weather",
        json!({
            "type": "object",
            "properties": {"city": {"type": "string"}},
            "required": ["city"],
            "additionalProperties": false
        }),
    )
    .description("Return the current weather for a city")
    .strict(true);
    let request = ChatCompletionRequest::new("glm-5.3")
        .message(ChatMessage::user("What is the weather in Shanghai?"))
        .tools(vec![Tool::function(weather)]);
    let response = client.chat_completion(&request).await?;
    println!("{response:#?}");
    Ok(())
}
