use std::fs;
use std::io::{self, Write};
use std::path::Path;
use std::sync::Arc;

use rustglm::{
    AgentHistoryPolicy, AgentManifest, AgentPersona, AgentRuntime, InMemoryVectorStore,
    SemanticMemory, ZhipuClient, ZhipuEmbeddingProvider,
};

fn read_line(prompt: &str) -> io::Result<String> {
    print!("{prompt}");
    io::stdout().flush()?;
    let mut value = String::new();
    io::stdin().read_line(&mut value)?;
    Ok(value.trim().to_owned())
}

fn value_or_default(value: String, default: &str) -> String {
    if value.is_empty() {
        default.to_owned()
    } else {
        value
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let api_key = match std::env::var("ZHIPU_API_KEY") {
        Ok(value) if !value.trim().is_empty() => value,
        _ => read_line("请输入智谱 API Key（输入内容会显示）: ")?,
    };
    if api_key.trim().is_empty() {
        return Err("API Key 不能为空".into());
    }

    let model = value_or_default(
        read_line("请输入模型名称，直接回车使用 glm-4-flash: ")?,
        "glm-4-flash",
    );
    let name = value_or_default(read_line("AI 角色名称，直接回车使用 小林: ")?, "小林");
    let role = value_or_default(
        read_line("AI 角色定位，直接回车使用 严谨且有个性的技术伙伴: ")?,
        "严谨且有个性的技术伙伴",
    );
    let style = value_or_default(
        read_line("表达风格，直接回车使用 自然、直接、避免空话: ")?,
        "自然、直接、避免空话",
    );
    let background = read_line("角色背景，可留空: ")?;
    let memory_mode = read_line("上下文模式 [0=不记忆, 1=最近消息, 2=语义向量记忆]: ")?;

    let persona = AgentPersona::new(name, role)
        .background(background)
        .speaking_style(style)
        .language("简体中文")
        .instruction("保持角色一致性，并明确区分事实、推断和不知道的信息")
        .boundary("不得伪造工具结果、来源或已经执行的操作");
    let mut manifest = AgentManifest::new(&model, persona);
    if memory_mode == "1" {
        manifest = manifest.history(AgentHistoryPolicy::Recent { max_messages: 20 });
    }

    let client = ZhipuClient::new(api_key)?;
    let mut vector_store = None;
    let mut vector_memory_path = None;
    let mut runtime = AgentRuntime::new(Arc::new(client.clone()), manifest)?;
    match memory_mode.as_str() {
        "" | "0" | "1" => {}
        "2" => {
            println!("语义向量记忆会额外调用 embedding-3，并可能产生费用");
            let path = value_or_default(
                read_line("向量记忆文件，直接回车使用 rustglm-memory.json: ")?,
                "rustglm-memory.json",
            );
            let embeddings = Arc::new(ZhipuEmbeddingProvider::new(client, "embedding-3"));
            let store = Arc::new(InMemoryVectorStore::new());
            if Path::new(&path).exists() {
                store.restore_json(&fs::read_to_string(&path)?)?;
                println!("已恢复 {} 条语义记忆", store.snapshot()?.len());
            }
            let memory = Arc::new(SemanticMemory::new(embeddings, store.clone()));
            runtime = runtime.semantic_memory(memory, 4)?;
            vector_store = Some(store);
            vector_memory_path = Some(path);
        }
        _ => return Err("上下文模式只能是 0、1 或 2".into()),
    }

    println!("已创建智能体，当前模型: {model}");
    println!("输入问题后回车发送，输入 clear 清空上下文，输入 exit 或 quit 退出");

    loop {
        let input = read_line("你: ")?;
        if input.eq_ignore_ascii_case("exit") || input.eq_ignore_ascii_case("quit") {
            break;
        }
        if input.eq_ignore_ascii_case("clear") {
            runtime.clear_history();
            runtime.clear_memory().await?;
            if let (Some(store), Some(path)) = (&vector_store, &vector_memory_path) {
                fs::write(path, store.snapshot_json()?)?;
            }
            println!("上下文已清空");
            continue;
        }
        if input.is_empty() {
            continue;
        }

        match runtime.run(&input).await {
            Ok(result) => {
                match result.response.text() {
                    Some(text) => println!("AI: {text}"),
                    None => eprintln!("请求成功，但响应没有文本内容: {:?}", result.response),
                }
                if let (Some(store), Some(path)) = (&vector_store, &vector_memory_path) {
                    fs::write(path, store.snapshot_json()?)?;
                }
            }
            Err(error) => eprintln!("请求失败: {error}"),
        }
    }

    Ok(())
}
