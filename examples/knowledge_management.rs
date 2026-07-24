use rustglm::{KnowledgeUpdateRequest, ZhipuClient};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let operation = args.next().unwrap_or_else(|| "list".to_owned());
    let client = ZhipuClient::new(std::env::var("ZHIPU_API_KEY")?)?;
    match operation.as_str() {
        "list" => println!("{:#?}", client.knowledge_bases(1, 20).await?),
        "capacity" => println!("{:#?}", client.knowledge_capacity().await?),
        "get" => {
            let id = args
                .next()
                .ok_or("knowledge_management get <knowledge-id>")?;
            println!("{:#?}", client.knowledge_base(&id).await?);
        }
        "update" => {
            let id = args
                .next()
                .ok_or("knowledge_management update <knowledge-id> <description>")?;
            let description = args
                .next()
                .ok_or("knowledge_management update <knowledge-id> <description>")?;
            let request = KnowledgeUpdateRequest {
                description: Some(description),
                ..KnowledgeUpdateRequest::default()
            };
            println!("{:#?}", client.update_knowledge_base(&id, &request).await?);
        }
        "delete" => {
            let id = args
                .next()
                .ok_or("knowledge_management delete <knowledge-id>")?;
            println!("{:#?}", client.delete_knowledge_base(&id).await?);
        }
        _ => return Err("operation must be list, capacity, get, update, or delete".into()),
    }
    Ok(())
}
