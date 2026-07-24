use std::path::Path;

use rustglm::{
    DocumentChunking, DocumentListQuery, RagDocumentUpload, ReEmbeddingRequest, UrlDocument,
    UrlDocumentUploadRequest, ZhipuClient,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let operation = args.next().unwrap_or_else(|| "list".to_owned());
    let client = ZhipuClient::new(std::env::var("ZHIPU_API_KEY")?)?;
    match operation.as_str() {
        "list" => {
            let knowledge_id = args
                .next()
                .ok_or("knowledge_documents list <knowledge-id>")?;
            println!(
                "{:#?}",
                client
                    .knowledge_documents(&DocumentListQuery::new(knowledge_id))
                    .await?
            );
        }
        "upload" => {
            let knowledge_id = args
                .next()
                .ok_or("knowledge_documents upload <knowledge-id> <file>")?;
            let input = args
                .next()
                .ok_or("knowledge_documents upload <knowledge-id> <file>")?;
            let path = Path::new(&input);
            let file_name = path
                .file_name()
                .and_then(|value| value.to_str())
                .ok_or("path must have a UTF-8 file name")?;
            let request = RagDocumentUpload::from_bytes(file_name, std::fs::read(path)?);
            println!(
                "{:#?}",
                client
                    .upload_knowledge_document(&knowledge_id, request)
                    .await?
            );
        }
        "url" => {
            let knowledge_id = args
                .next()
                .ok_or("knowledge_documents url <knowledge-id> <url>")?;
            let url = args
                .next()
                .ok_or("knowledge_documents url <knowledge-id> <url>")?;
            let request = UrlDocumentUploadRequest {
                knowledge_id,
                upload_detail: vec![UrlDocument::new(url, DocumentChunking::Heading)],
            };
            println!("{:#?}", client.upload_knowledge_urls(&request).await?);
        }
        "get" => {
            let id = args.next().ok_or("knowledge_documents get <document-id>")?;
            println!("{:#?}", client.knowledge_document(&id).await?);
        }
        "images" => {
            let id = args
                .next()
                .ok_or("knowledge_documents images <document-id>")?;
            println!("{:#?}", client.knowledge_document_images(&id).await?);
        }
        "reembed" => {
            let id = args
                .next()
                .ok_or("knowledge_documents reembed <document-id>")?;
            println!(
                "{:#?}",
                client
                    .reembed_knowledge_document(&id, &ReEmbeddingRequest::default())
                    .await?
            );
        }
        "delete" => {
            let id = args
                .next()
                .ok_or("knowledge_documents delete <document-id>")?;
            println!("{:#?}", client.delete_knowledge_document(&id).await?);
        }
        _ => {
            return Err(
                "operation must be list, upload, url, get, images, reembed, or delete".into(),
            );
        }
    }
    Ok(())
}
