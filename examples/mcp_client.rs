use nextjson::Map;
use rustglm::McpClientConfig;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let endpoint = args.next().ok_or(
        "usage: cargo run --example mcp_client --features mcp -- <endpoint> [list|call|read|prompt]",
    )?;
    let operation = args.next().unwrap_or_else(|| "list".to_owned());
    let mut config = McpClientConfig::new(endpoint);
    if let Ok(token) = std::env::var("MCP_BEARER_TOKEN") {
        config = config.bearer_token(token);
    }
    let mut client = config.connect().await?;
    match operation.as_str() {
        "list" => {
            for tool in client.list_tools().await? {
                println!("tool={}", tool.name);
            }
            for resource in client.list_resources().await? {
                println!("resource={}", resource.uri);
            }
            for template in client.list_resource_templates().await? {
                println!("resource_template={}", template.uri_template);
            }
            for prompt in client.list_prompts().await? {
                println!("prompt={}", prompt.name);
            }
        }
        "call" => {
            let name = args
                .next()
                .ok_or("mcp_client <endpoint> call <tool> [json]")?;
            let arguments = args
                .next()
                .map(|raw| nextjson::from_str::<Map>(&raw))
                .transpose()?;
            println!("{:#?}", client.call_tool(name, arguments).await?);
        }
        "read" => {
            let uri = args.next().ok_or("mcp_client <endpoint> read <uri>")?;
            println!("{:#?}", client.read_resource(uri).await?);
        }
        "prompt" => {
            let name = args.next().ok_or("mcp_client <endpoint> prompt <name>")?;
            println!("{:#?}", client.get_prompt(name, None).await?);
        }
        _ => return Err("operation must be list, call, read, or prompt".into()),
    }
    client.close().await?;
    Ok(())
}
