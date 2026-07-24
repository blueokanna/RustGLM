use rustglm::{KnowledgeCreateRequest, KnowledgeEmbeddingModel, ZhipuClient};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let api_key = std::env::var("ZHIPU_API_KEY")?;
    let name = std::env::var("RUSTGLM_KNOWLEDGE_NAME")?;
    let client = ZhipuClient::new(api_key)?;
    let response = client
        .create_knowledge_base(&KnowledgeCreateRequest::new(
            name,
            KnowledgeEmbeddingModel::Embedding3Pro,
        ))
        .await?;
    let knowledge = response.data.ok_or("knowledge service returned no data")?;
    println!("knowledge_id={}", knowledge.id);
    Ok(())
}
