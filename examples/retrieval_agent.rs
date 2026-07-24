use futures_util::StreamExt;
use rustglm::{RetrievalAgentConfig, RetrievalAgentMessage, RetrievalAgentRequest, ZhipuClient};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let knowledge_id = args
        .next()
        .ok_or("usage: retrieval_agent <knowledge-id> [question]")?;
    let question = args
        .next()
        .unwrap_or_else(|| "Summarize the relevant runbook.".to_owned());
    let request = RetrievalAgentRequest::new(RetrievalAgentConfig::new([knowledge_id]))
        .message(RetrievalAgentMessage::user(question));
    let client = ZhipuClient::new(std::env::var("ZHIPU_API_KEY")?)?;
    let mut stream = client.retrieval_agent_stream(&request, None).await?;
    while let Some(event) = stream.next().await {
        let event = event?;
        if let Some(text) = event.text() {
            print!("{text}");
        }
    }
    println!();
    Ok(())
}
