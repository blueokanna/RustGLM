use futures_util::StreamExt;
use rustglm::{
    AgentAsyncResultRequest, AgentConversationRequest, OfficialAgentMessage, OfficialAgentRequest,
    ZhipuClient,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let operation = args.next().unwrap_or_else(|| "stream".to_owned());
    let agent_id = args
        .next()
        .unwrap_or_else(|| "general_translation".to_owned());
    let client = ZhipuClient::new(std::env::var("ZHIPU_API_KEY")?)?;
    match operation.as_str() {
        "stream" => {
            let prompt = args.next().unwrap_or_else(|| "Translate: hello".to_owned());
            let request =
                OfficialAgentRequest::new(agent_id).message(OfficialAgentMessage::user(prompt));
            let mut stream = client.official_agent_stream(&request).await?;
            while let Some(event) = stream.next().await {
                println!("{:#?}", event?);
            }
        }
        "async-result" => {
            let async_id = args
                .next()
                .ok_or("official_agent_lifecycle async-result <agent-id> <async-id>")?;
            let response = client
                .official_agent_async_result(&AgentAsyncResultRequest { async_id, agent_id })
                .await?;
            println!("{response:#?}");
        }
        "conversation" => {
            let conversation_id = args
                .next()
                .ok_or("official_agent_lifecycle conversation <agent-id> <conversation-id>")?;
            let response = client
                .official_agent_conversation(&AgentConversationRequest {
                    agent_id,
                    conversation_id,
                    custom_variables: None,
                })
                .await?;
            println!("{response:#?}");
        }
        _ => return Err("operation must be stream, async-result, or conversation".into()),
    }
    Ok(())
}
