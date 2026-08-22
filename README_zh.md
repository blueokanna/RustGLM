# RustGLM

[English](README.md) | 简体中文

[![CI](https://github.com/blueokanna/rustglm/actions/workflows/ci.yml/badge.svg)](https://github.com/blueokanna/rustglm/actions/workflows/ci.yml)
[![Release](https://github.com/blueokanna/rustglm/actions/workflows/release.yml/badge.svg)](https://github.com/blueokanna/rustglm/actions/workflows/release.yml)
[![Crates.io](https://img.shields.io/crates/v/rustglm.svg)](https://crates.io/crates/rustglm)
[![Documentation](https://docs.rs/rustglm/badge.svg)](https://docs.rs/rustglm)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)

RustGLM 是面向智谱 AI 开放平台的异步 Rust SDK。除了常规的聊天能力（GLM-5 强类型请求、SSE 流式、ToolStream 聚合），它还覆盖批量任务、知识库、文件解析与 OCR、图像/视频生成、语音、双向 Realtime WebSocket、带语义记忆的本地 Agent 运行时，以及基于 `rmcp` 的独立 MCP 客户端。

这个库是给后端服务用的，所以它不会碰你的机器：不发现配置文件、不做隐式磁盘写入、不上报遥测。凭据、重试、持久化、客户端生命周期，全部由你掌控。

## 快速开始

```toml
[dependencies]
rustglm = "1.0"
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

```powershell
$env:ZHIPU_API_KEY = "key_id.secret"
cargo run --example chat_completion
```

```rust,no_run
use rustglm::{ChatCompletionRequest, ChatMessage, ZhipuClient};

# async fn run() -> rustglm::Result<()> {
let client = ZhipuClient::new("key_id.secret")?;
let request = ChatCompletionRequest::new("glm-5.3")
    .message(ChatMessage::user("用一段话解释 Rust 所有权。"));
let response = client.chat_completion(&request).await?;
println!("{}", response.text().unwrap_or_default());
# Ok(())
# }
```

## 要求

- Rust 1.88 或更高版本（2024 edition）
- 异步执行需要 Tokio
- 智谱 API Key，或你已有的 Bearer token

## 这个库做什么、不做什么

几条底线，避免在生产环境里出现意外：

- 不做隐式磁盘 I/O。文件与 RAG 上传接收调用方持有的字节数据，库本身从不打开路径。
- 不隐式读取凭据。API Key 是构造参数；`EnvironmentSecretResolver` 是需要显式启用的 Agent 工具，不是魔法。
- 不产生你没要求的网络流量。构造配置值零 I/O，只有当你 await 端点方法时才会发请求。
- 不上报遥测、不访问元数据端点、不用 NTP。JWT 签名只读本机时钟。
- HTTP 重试默认关闭。即便你开启，也只有幂等方法（GET、HEAD、DELETE、OPTIONS、PUT）会在连接/超时失败时自动重试，避免把已经落到服务端的 POST 再打一遍。
- 凭据只在 TLS 上传输。非本机地址的明文 `http://` 基址默认拒绝，除非 `HttpConfig::allow_insecure(true)` 显式开启——与 MCP、Realtime 的策略一致。

## Feature flags

默认 feature 覆盖完整的智谱能力面；MCP 客户端因为引入 `rmcp` 而默认不启用。

| Feature | 默认启用 | 能力 |
| --- | ---: | --- |
| `agents` | 是 | 官方 Agent、Assistant 端点、本地 Agent 运行时 |
| `audio` | 是 | GLM-4-Voice、转录、语音、音色管理 |
| `batch` | 是 | 强类型 Batch API：创建、列表、查询、取消 |
| `files` | 是 | 文件上传/下载/删除、解析、OCR、版面分析 |
| `images` | 是 | 图像生成 |
| `mcp` | 否 | 独立 Streamable HTTP MCP 客户端（`rmcp`） |
| `rag` | 是 | Retrieval Agent、知识库与文档管理 |
| `realtime` | 是 | 强类型双向 WebSocket 客户端 |
| `tools` | 是 | 托管工具类型、Web 操作、ToolStream 聚合 |
| `video` | 是 | 视频生成 |
| `full` | 否 | 全部启用，包括 `mcp` |

最小聊天构建：

```toml
[dependencies]
rustglm = { version = "1.0", default-features = false }
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

需要更多时按需选取：

```toml
[dependencies]
rustglm = {
    version = "1.0",
    default-features = false,
    features = ["batch", "mcp", "rag", "realtime", "tools"]
}
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

## 认证

`ZhipuClient::new`、`ZhipuConfig::new` 和 `RealtimeConfig::new` 都直接接收凭据。

- `key_id.secret` 视为组合 API Key，签名为 HS256 JWT（带缓存，过期前自动刷新）。
- 其他任何非空值视为不透明 Bearer token。
- 需要显式指定时用 `ZhipuAuthentication::jwt` 或 `ZhipuAuthentication::bearer`。

不要提交凭据。从环境变量、密钥管理器或工作负载身份提供方读取，这部分由你负责。

## 强类型 GLM-5 聊天

标记类型与封闭能力 trait 让强类型 API 在编译期就拒绝不支持的字段。你没法给不支持推理强度的模型发 `reasoning_effort`，请求在包含用户或工具输入之前也无法触达网络。

```rust,no_run
use rustglm::{Glm53, ReasoningEffort, Thinking, TypedChatRequest, ZhipuClient};

# async fn run() -> rustglm::Result<()> {
let client = ZhipuClient::new("key_id.secret")?;
let request = TypedChatRequest::<Glm53>::new()
    .system("Answer with evidence.")
    .thinking(Thinking::enabled())
    .reasoning_effort(ReasoningEffort::High)
    .user("Summarize the incident report.");
let response = client.typed_chat_completion(&request).await?;
println!("{}", response.text().unwrap_or_default());
# Ok(())
# }
```

支持的标记类型见下表。不在表里的模型（新发布、私有部署）直接用 `ChatCompletionRequest` 传模型 ID 字符串即可。

| 文本模型 | 标记类型 | Thinking | Reasoning effort | ToolStream |
| --- | --- | :---: | :---: | :---: |
| `glm-5.3` | `Glm53` | 是 | 是 | 是 |
| `glm-5.2` | `Glm52` | 是 | 是 | 是 |
| `glm-5.1` | `Glm51` | 是 | 否 | 是 |
| `glm-5.1-highspeed` | `Glm51Highspeed` | 是 | 否 | 是 |
| `glm-5-turbo` | `Glm5Turbo` | 是 | 否 | 是 |
| `glm-5` | `Glm5` | 是 | 否 | 是 |
| `glm-4.7` | `Glm47` | 是 | 否 | 是 |
| `glm-4.7-flash` | `Glm47Flash` | 是 | 否 | 否 |
| `glm-4.7-flashx` | `Glm47FlashX` | 是 | 否 | 否 |
| `glm-4.6` | `Glm46` | 是 | 否 | 是 |
| `glm-4.5-air` | `Glm45Air` | 是 | 否 | 否 |
| `glm-4.5-airx` | `Glm45AirX` | 是 | 否 | 否 |
| `glm-4.5-flash` | `Glm45Flash` | 是 | 否 | 否 |
| `glm-4-flash-250414` | `Glm4Flash250414` | 否 | 否 | 否 |
| `glm-4-flashx-250414` | `Glm4FlashX250414` | 否 | 否 | 否 |
| `glm-4-long` | `Glm4Long` | 否 | 否 | 否 |
| `charglm-4` | `Charglm4` | 否 | 否 | 否 |
| `emohaa` | `Emohaa` | 否 | 否 | 否 |

| 视觉模型 | 标记类型 | Thinking | ToolStream |
| --- | --- | :---: | :---: |
| `glm-5v-turbo` | `Glm5vTurbo` | 是 | 否 |
| `autoglm-phone` | `AutoGlmPhone` | 否 | 否 |
| `glm-4.6v` | `Glm46v` | 是 | 否 |
| `glm-4.6v-flash` | `Glm46vFlash` | 是 | 否 |
| `glm-4.6v-flashx` | `Glm46vFlashX` | 是 | 否 |
| `glm-4v-flash` | `Glm4vFlash` | 否 | 否 |
| `glm-4.1v-thinking-flash` | `Glm41vThinkingFlash` | 是 | 否 |
| `glm-4.1v-thinking-flashx` | `Glm41vThinkingFlashX` | 是 | 否 |
| `glm-ocr` | `GlmOcr` | 否 | 否 |
| `glm-4.1v-thinking` | `Glm41vThinking` | 是 | 否 |

`ReasoningEffort`、`Thinking`、ToolStream、工具和视觉输入只存在于声明了对应能力的标记类型上，不支持的字段走不到传输层。

## ToolStream

ToolStream 把碎片化的 SSE 函数调用增量合并成完整强类型调用，同时保留文本、推理、用量与流错误。它还有合理上限：单次调用的参数与并发未完成调用数都有界，参数不是合法 JSON 的调用不会被当作"已完成"发出。

```rust,no_run
use futures_util::StreamExt;
use rustglm::{Glm53, ToolStreamEvent, TypedChatRequest, ZhipuClient};

# async fn run() -> rustglm::Result<()> {
let client = ZhipuClient::new("token")?;
let request = TypedChatRequest::<Glm53>::new().tool_stream().user("Check the deployment status.");
let mut stream = client.typed_chat_tool_stream(&request).await?;
while let Some(event) = stream.next().await {
    if let ToolStreamEvent::ToolCallCompleted(call) = event? {
        println!("{} {}", call.name, call.arguments);
    }
}
# Ok(())
# }
```

## 本地 Agent 运行时

除了官方托管 Agent，还有一个自托管的 `AgentRuntime`：按 manifest 驱动的循环，跑在任意 `ChatProvider` 之上——persona 提示词、历史策略、注册工具、可选语义记忆。这个运行时默认就是可防御的：

- `max_steps` 有上限；工具总执行次数、单次工具输出字节数都有预算。
- 可选的 `run_timeout` 与 `tool_timeout` 让卡死的模型或挂起的工具不会永远阻塞请求。
- 召回的记忆以"不可信上下文"注入，明确标注为数据而非指令——限制进入记忆库的注入内容能造成的影响面。
- 错误是结构化的（`StepLimit`、`BudgetExceeded`、`NoOutput`、`ToolError::NotRegistered` 等），可以直接分支处理，不用字符串匹配。

```rust,no_run
use std::sync::Arc;
use rustglm::{AgentManifest, AgentPersona, AgentRuntime, ZhipuClient};

# async fn run() -> rustglm::Result<()> {
let client = Arc::new(ZhipuClient::new("token")?);
let manifest = AgentManifest::new(
    "glm-5.3",
    AgentPersona::new("Lin", "technical companion").language("English"),
);
let mut agent = AgentRuntime::new(client, manifest)?;
let result = agent.run("What is the status?").await?;
# let _ = result;
# Ok(())
# }
```

## 内容安全

`moderate_content` 把强类型的文本、图片、音频或视频内容发给内容安全模型，返回结构化风险结果。文本输入上限 2000 字符；媒体 URL 必须是绝对 HTTP(S) 地址，在发出任何请求前校验。

```rust,no_run
use rustglm::{ModerationItem, ModerationRequest, ZhipuClient};

# async fn run() -> rustglm::Result<()> {
let client = ZhipuClient::new("token")?;
let response = client
    .moderate_content(&ModerationRequest::new_items([
        ModerationItem::text("Check this comment"),
        ModerationItem::image_url("https://example.com/upload.png"),
    ]))
    .await?;
for result in response.result_list.unwrap_or_default() {
    println!("{:?} -> {:?}", result.content_type, result.risk_level);
}
# Ok(())
# }
```

## Batch API

`batch` feature 提供强类型补全窗口和状态。Batch 输入文件通过 `files` API 显式上传。

```rust,no_run
use rustglm::{BatchCreateRequest, ZhipuClient};

# async fn run() -> rustglm::Result<()> {
let client = ZhipuClient::new("token")?;
let request = BatchCreateRequest::new("input-file-id", "/v4/chat/completions");
let batch = client.create_batch(&request).await?;
let current = client.batch(&batch.id).await?;
println!("{:?}", current.status);
# Ok(())
# }
```

可用方法为 `create_batch`、`batches`、`batch` 和 `cancel_batch`。不在 `1..=100` 范围内的列表限制会在任何网络 I/O 之前返回 `BatchError::InvalidLimit`。

## 知识库与 RAG

`rag` feature 遵循官方知识库 OpenAPI 路径：知识库 CRUD、容量、检索、文档列表/详情、内存文件上传、URL 摄取、删除、文档图片与重新嵌入。上传和回调 URL 在发出前都会校验——必须是带主机的绝对 HTTP(S) URL，且不允许内嵌凭据。

```rust,no_run
use rustglm::{KnowledgeCreateRequest, KnowledgeEmbeddingModel, ZhipuClient};

# async fn run() -> rustglm::Result<()> {
let client = ZhipuClient::new("token")?;
let created = client.create_knowledge_base(&KnowledgeCreateRequest::new(
    "engineering-runbooks", KnowledgeEmbeddingModel::Embedding3Pro,
)).await?;
println!("{}", created.data.expect("successful response").id);
# Ok(())
# }
```

`RagDocumentUpload::from_bytes` 不提供基于路径的构造函数是有意为之：文件读取、大小限制、加密、租户边界与保留策略都留在你手里。

## MCP 客户端

`mcp` feature 是独立的 Model Context Protocol 客户端（与在模型请求里配置托管 MCP 工具的 `McpTool` 不同）。协议帧、初始化、工具、资源、提示词和 Streamable HTTP 传输来自官方 `rmcp` crate。

```rust,no_run
use rustglm::McpClientConfig;

# async fn run() -> rustglm::Result<()> {
let mut client = McpClientConfig::new("https://mcp.example.com/mcp")
    .bearer_token("tenant-token").header("x-tenant-id", "acme")?.connect().await?;
for tool in client.list_tools().await? { println!("{}", tool.name); }
client.close().await?;
# Ok(())
# }
```

值得知道的安全默认值：

- 只接受绝对 HTTP(S) 端点。非本机地址的明文 `http://` 默认拒绝，除非显式 `allow_insecure(true)`。
- 内嵌凭据的 URL 直接拒绝。
- `Debug` 输出脱敏 Bearer token，headers 只打印名字、不打印值。
- 禁用重定向，SDK 创建的客户端带连接/请求超时。
- SSE 重试和过期会话自动初始化默认关闭。
- 可以注入自己的 `reqwest::Client` 控制代理、TLS、DNS 或策略。

## Realtime WebSocket

`realtime` feature 是强类型双向 WebSocket 客户端。音视频以调用方持有的字节切片传入，在内存中编码。

```rust,no_run
use rustglm::{RealtimeConfig, RealtimeRequest, TypedRealtimeSession};

# async fn run() -> rustglm::Result<()> {
let mut connection = RealtimeConfig::new("token").connect().await?;
let session = TypedRealtimeSession::default().instructions("Be concise.").server_vad();
connection.send_request(&RealtimeRequest::session_update(session)?).await?;
connection.send_request(&RealtimeRequest::append_audio(&[0_u8; 320])?).await?;
while let Some(event) = connection.next_typed_event().await {
    if let Some(text) = event?.delta_text() { print!("{text}"); }
}
# Ok(())
# }
```

传输层的加固：

- 默认端点是 `wss://`。明文 `ws://` 默认拒绝，除非显式 `allow_insecure(true)`。
- 消息与帧大小限制在 WebSocket 层强制执行。
- 事件循环不会阻塞在已满的出站通道上——消费慢就丢事件，而不是把连接、心跳和关闭全部卡死。
- 写入有时间上限，`close()` 等待后台任务最多几秒。

该客户端还支持强类型会话工具、函数调用输出、响应选项、转录会话、客户端/服务端 VAD、取消、音频提交/清空、视频帧与显式关闭。

## 错误

`SdkError` 是唯一的公共错误类型。领域错误保持为显式枚举，可以直接匹配，不用解析字符串。

```rust
use rustglm::{BatchError, SdkError};

fn classify(error: SdkError) {
    match error {
        SdkError::Batch(BatchError::InvalidLimit(limit)) => eprintln!("invalid batch limit: {limit}"),
        SdkError::Api(api) => eprintln!("HTTP {} request_id={:?}", api.status, api.request_id),
        SdkError::PayloadTooLarge { limit, .. } => eprintln!("response exceeded {limit} bytes"),
        other => eprintln!("{other}"),
    }
}
```

封装区分配置、校验、传输、超时、API、解码、流、WebSocket、不支持能力、Agent、工具、Batch、RAG 和 MCP 失败。`ApiError` 保留 HTTP 状态、厂商代码、消息与请求 ID，也保留原始响应体——调试很有用，但响应体可能包含模型输出等敏感数据，记录前请三思。

## HTTP 策略

`HttpConfig` 控制请求超时、连接超时、连接池空闲超时、响应体大小上限、user agent、默认请求头、重试策略，以及可选的调用方构建 `reqwest::Client`。

- 重试默认关闭；即便开启，也只重试你配置的状态码，且连接/超时重试仅对幂等方法生效。
- 响应体按上限读取（`max_response_bytes`，默认 64 MiB），防止异常或恶意的端点撑爆进程内存。错误响应体另有 64 KiB 独立上限。同样的思路也用于 SSE 事件（单事件 16 MiB、单事件最多 4096 行 data）和流式工具参数（单调用 1 MiB）。
- 非本机地址的基址必须是 HTTPS。明文 `http://` 只允许回环与私网地址（本地测试、内网代理），其余地址需要 `allow_insecure(true)`。
- JWT 令牌缓存使用单调时钟，系统时间回拨不会延长已过期令牌的使用寿命。
- 进入 `ApiError` 的错误体先经过脱敏：智谱 `id.secret` 格式的 Key、Bearer token、以及配置的凭据本身都会替换为 `[FILTERED]`，避免密钥通过日志外泄。

## API 覆盖范围

公开能力速查。接受 `nextjson::Value` 的方法是有意为之——服务商 Schema 演化比这个 crate 发版快。

| 能力领域 | Feature | 公开方法 |
| --- | --- | --- |
| 聊天与流 | 核心；ToolStream 需 `tools` | `chat_completion`、`chat_completion_stream`、`chat_tool_stream`、`typed_chat_completion`、`typed_chat_completion_stream`、`typed_chat_tool_stream` |
| 异步与向量 API | 核心 | `async_chat`、`async_result`、`embedding`、`rerank`、`tokenizer` |
| 图像与视频 | `images`、`video` | `create_image`、`create_image_async`、`create_video` |
| 音频与音色 | `audio` | `glm_4_voice`、`transcribe`、`speech`、`clone_voice`、`voices`、`delete_voice` |
| 托管工具 | `tools` | `web_search`、`read_web_page`、`moderate`、`moderate_content` |
| 文件与文档处理 | `files` | `upload_file`、`files`、`file_content`、`delete_file`、`create_file_parse_task`、`file_parse_result`、`parse_file_sync`、`ocr`、`parse_layout` |
| Batch | `batch` | `create_batch`、`batches`、`batch`、`cancel_batch` |
| 官方 Agent 与 Assistant | `agents` | `official_agent`、`official_agent_stream`、`official_agent_async_result`、`official_agent_conversation`、`assistant`、`assistants`、`assistant_conversations` |
| 知识库与检索 | `rag` | `create_knowledge_base`、`knowledge_bases`、`knowledge_base`、`update_knowledge_base`、`delete_knowledge_base`、`knowledge_capacity`、`retrieve_knowledge`、`knowledge_documents`、`upload_knowledge_document`、`upload_knowledge_urls`、`knowledge_document`、`delete_knowledge_document`、`knowledge_document_images`、`reembed_knowledge_document`、`retrieval_agent_stream` |
| 通用协议入口 | 核心 | `ZhipuClient` 与 `OpenAiCompatibleClient` 上的 `request_json` |
| 独立 MCP | `mcp` | `McpClientConfig::connect`，以及由 `rmcp` 提供的强类型工具、资源、提示词和 Streamable HTTP 操作 |
| Realtime | `realtime` | `RealtimeConfig::connect`、强类型请求/事件、VAD、媒体缓冲、函数调用输出、取消与显式关闭 |

服务商新字段还没上强类型 builder 时用 `ChatCompletionRequest`。`request_json` 只接受配置 Base URL 下的相对路径——绝对 URL 和 `..` 路径段都会被拒绝。

## 示例

`examples/` 下有 36 个可运行示例，每个端点领域一个聚焦示例，外加几个把常用操作串起来的生命周期示例。`cargo check --all-targets --all-features` 可以在不联网的情况下编译检查全部示例。

### 聊天、模型与向量

| 示例 | 演示内容 |
| --- | --- |
| [`chat_completion`](examples/chat_completion.rs) | `chat_completion` |
| [`chat_stream`](examples/chat_stream.rs) | `chat_completion_stream` |
| [`typed_chat`](examples/typed_chat.rs) | `typed_chat_completion`、Thinking、推理强度 |
| [`multimodal_chat`](examples/multimodal_chat.rs) | 视觉内容片段与图片 URL 输入 |
| [`function_calling`](examples/function_calling.rs) | 函数 Schema 与 `Tool::function` |
| [`tool_stream`](examples/tool_stream.rs) | `typed_chat_tool_stream` 与聚合后的函数调用增量 |
| [`async_chat`](examples/async_chat.rs) | `async_chat`、`async_result` |
| [`embedding`](examples/embedding.rs) | `EmbeddingRequest`、`embedding` |
| [`rerank`](examples/rerank.rs) | `RerankRequest`、`rerank` |
| [`tokenizer`](examples/tokenizer.rs) | `TokenizerRequest`、`tokenizer` |
| [`openai_compatible`](examples/openai_compatible.rs) | `OpenAiCompatibleConfig`、`ChatProvider` |

### 媒体、文件与文档处理

| 示例 | 演示内容 |
| --- | --- |
| [`image_generation`](examples/image_generation.rs) | `create_image`、`create_image_async` |
| [`video_generation`](examples/video_generation.rs) | `create_video`、异步任务 ID |
| [`speech`](examples/speech.rs) | `SpeechRequest`、`speech` |
| [`transcription`](examples/transcription.rs) | `TranscriptionRequest`、`transcribe` |
| [`glm_4_voice`](examples/glm_4_voice.rs) | GLM-4-Voice 输入与 WAV 输出 |
| [`voice_management`](examples/voice_management.rs) | `clone_voice`、`voices`、`delete_voice` |
| [`file_management`](examples/file_management.rs) | `upload_file`、`files`、`file_content`、`delete_file` |
| [`file_parsing`](examples/file_parsing.rs) | `create_file_parse_task`、`file_parse_result`、`parse_file_sync` |
| [`document_understanding`](examples/document_understanding.rs) | `ocr`、`parse_layout` |

### Batch、托管工具与 RAG

| 示例 | 演示内容 |
| --- | --- |
| [`web_search`](examples/web_search.rs) | `web_search` |
| [`hosted_tools`](examples/hosted_tools.rs) | `read_web_page`、`moderate` |
| [`file_batch`](examples/file_batch.rs) | 上传 JSONL 并调用 `create_batch` |
| [`batch_management`](examples/batch_management.rs) | Batch 创建、列表、查询和取消 |
| [`knowledge_base`](examples/knowledge_base.rs) | `create_knowledge_base` |
| [`knowledge_management`](examples/knowledge_management.rs) | 知识库列表、详情、更新、容量与删除 |
| [`knowledge_documents`](examples/knowledge_documents.rs) | 文档列表、上传、URL 导入、详情、图片、重嵌入与删除 |
| [`knowledge_retrieval`](examples/knowledge_retrieval.rs) | `retrieve_knowledge` |
| [`retrieval_agent`](examples/retrieval_agent.rs) | `retrieval_agent_stream` |

### Agent、MCP 与 Realtime

| 示例 | 演示内容 |
| --- | --- |
| [`official_agent`](examples/official_agent.rs) | 强类型官方 Agent v1 调用 |
| [`official_agent_lifecycle`](examples/official_agent_lifecycle.rs) | Agent 流、异步结果与会话操作 |
| [`assistants`](examples/assistants.rs) | Assistant 调用、列表与会话 |
| [`custom_agent`](examples/custom_agent.rs) | 带应用工具的本地 Agent 运行时 |
| [`interactive_chat`](examples/interactive_chat.rs) | 多轮运行时与可选语义记忆 |
| [`mcp_client`](examples/mcp_client.rs) | MCP 工具、资源、提示词与关闭连接 |
| [`realtime_audio_video`](examples/realtime_audio_video.rs) | Realtime PCM/WAV、可选 JPEG 帧与强类型事件 |

通过 `cargo run --example <name> -- <参数>` 运行示例。MCP 示例需要 feature：`cargo run --example mcp_client --features mcp -- <endpoint>`。大多数智谱示例需要 `ZHIPU_API_KEY`；`openai_compatible` 使用 `OPENAI_COMPATIBLE_BASE_URL` 与 `OPENAI_COMPATIBLE_API_KEY`。运行示例会消耗额度、创建或删除远程资源——别随手拿生产账号跑。

## CI 与发布

CI 会跑格式化、将警告视为错误的 Clippy、无默认/默认/全部/各企业 feature 构建、测试与 doctest、将警告视为错误的文档构建、从已提交 lockfile 构建包，并在全 feature 构建上强制 90% 行覆盖率，产物含 LCOV 与文本摘要。

发布工作流在 `v*` tag 推送时运行，也可以手动触发：在 Actions 页面选一个提交或分支，在 `tag` 输入框填 `v<Cargo.toml version>`。工作流检出所选版本而不是假定 tag 已存在；它会拒绝版本不匹配、跑完所有发布门禁、构建 `.crate`、写入 `SHA256SUMS`、创建缺失的附注 tag（已有 tag 只有指向已验证提交时才接受），最后在配置 `CARGO_REGISTRY_TOKEN` 时可选发布 crates.io 并创建/更新 GitHub Release。

```bash
# 先更新 Cargo.toml 版本号，然后：
git tag -s v1.0.0 -m "RustGLM v1.0.0"
git push origin v1.0.0
```

仓库里不保存 API Key 或 registry token。`CARGO_REGISTRY_TOKEN` 只应该以 GitHub Actions secret 的形式存在。

## 测试与覆盖率

```bash
cargo fmt --all -- --check
cargo test --all-targets --no-default-features
cargo test --all-targets
cargo test --all-targets --all-features
cargo clippy --all-targets --all-features -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --all-features --no-deps
cargo package --locked
```

CI 强制 90% 行覆盖率，每次运行都会发布 LCOV 与文本摘要产物。本地复现：

```bash
cargo coverage       # 输出摘要并检查 90% 行覆盖率门槛
cargo coverage-lcov  # 生成 target/rustglm-lcov.info，并检查相同门槛
```

各模块完整数据见 [docs/COVERAGE.md](docs/COVERAGE.md)——判断某个提交的覆盖率请以 CI 产物为准，别拿过期的快照当结论。2 个真实服务测试（`live_zhipu`、`live_realtime`）默认忽略，需要真实 Key 时再显式运行：

```powershell
$env:ZHIPU_API_KEY = "key_id.secret"
cargo test --test live_zhipu -- --ignored --nocapture
cargo test --test live_realtime -- --ignored --nocapture
```

## 许可证

Apache License 2.0。参见 [LICENSE](LICENSE)。
