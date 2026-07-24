use rustglm::{KnowledgeRetrieveRequest, RecallMethod, ZhipuClient};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let knowledge_id = args
        .next()
        .ok_or("usage: knowledge_retrieval <knowledge-id> [query]")?;
    let query = args
        .next()
        .unwrap_or_else(|| "What are the rollback steps?".to_owned());
    let mut request = KnowledgeRetrieveRequest::new(query, [knowledge_id]);
    request.top_k = Some(10);
    request.top_n = Some(5);
    request.recall_method = Some(RecallMethod::Mixed);
    let client = ZhipuClient::new(std::env::var("ZHIPU_API_KEY")?)?;
    let response = client.retrieve_knowledge(&request).await?;
    for item in response.data.unwrap_or_default() {
        println!("score={} text={}", item.score, item.text);
    }
    Ok(())
}
