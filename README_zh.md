# RustGLM

[English](README.md) | 简体中文

[![CI](https://github.com/blueokanna/rustglm/actions/workflows/ci.yml/badge.svg)](https://github.com/blueokanna/rustglm/actions/workflows/ci.yml)
[![Release](https://github.com/blueokanna/rustglm/actions/workflows/release.yml/badge.svg)](https://github.com/blueokanna/rustglm/actions/workflows/release.yml)
[![Crates.io](https://img.shields.io/crates/v/rustglm.svg)](https://crates.io/crates/rustglm)
[![Documentation](https://docs.rs/rustglm/badge.svg)](https://docs.rs/rustglm)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)

RustGLM 是面向智谱 AI 开放平台的非官方异步 Rust SDK，提供强类型 GLM-5 请求、SSE 和 ToolStream 聚合、双向 Realtime WebSocket 会话、Batch API 操作、知识库管理，以及基于官方 Rust MCP SDK 的 MCP 客户端。

本 crate 面向生产后端。网络策略、持久化、凭据、重试和客户端生命周期均由应用显式控制。

## 快速开始

添加默认 SDK 与 Tokio 运行时：

```toml
[dependencies]
rustglm = "1.0.0"
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

设置凭据并运行补全示例：

```powershell
$env:ZHIPU_API_KEY = "key_id.secret"
cargo run --example chat_completion
```

```rust,no_run
use rustglm::{ChatCompletionRequest, ChatMessage, ZhipuClient};

# async fn run() -> rustglm::Result<()> {
let client = ZhipuClient::new("key_id.secret")?;
let request = ChatCompletionRequest::new("glm-5.2")
    .message(ChatMessage::user("用一段话解释 Rust 所有权。"));
let response = client.chat_completion(&request).await?;
println!("{}", response.text().unwrap_or_default());
# Ok(())
# }
```

## 要求

- Rust 1.88 或更高版本
- Rust 2024 Edition
- 用于异步执行的 Tokio
- 智谱 API Key，或已签发的智谱 Bearer token

Cargo 包名和 Rust crate 名均为 `rustglm`。

## 副作用约定

RustGLM 不会进行隐式磁盘 I/O。

- 库不会创建目录、查找配置文件、写入日志、缓存响应、持久化对话或保存音视频。
- 文件与 RAG 上传 API 接收调用方拥有的字节数据，内部绝不打开文件路径。
- 响应、SSE 帧、Realtime 媒体、内存快照和工具事件始终保留在内存或异步流中。
- 库不会从环境变量读取 API Key；凭据由构造函数传入。`EnvironmentSecretResolver` 是显式启用的 Agent 工具。
- 库从不访问 NTP、元数据、遥测或模型发现服务；JWT 签名仅使用本机系统时钟。
- HTTP 默认重试次数为零；MCP SSE 重试和过期会话自动初始化默认关闭。
- 构造配置值不产生网络 I/O。智谱请求只会在等待端点方法时发出；MCP 或 Realtime 连接只会在等待 `connect` 时建立。

示例可能显式读取环境变量或本地文件。这些属于应用层行为，并非 SDK 执行。

## Feature flags

默认 feature 保留广泛的智谱 API 能力，同时使独立 MCP 协议客户端保持按需启用。

| Feature | 默认启用 | API 能力 |
| --- | ---: | --- |
| `agents` | 是 | 官方 Agent、Assistant 端点和本地 Agent 运行时 |
| `audio` | 是 | GLM-4-Voice、转录、语音和音色操作 |
| `batch` | 是 | 强类型 Batch API 创建、列表、查询和取消 |
| `files` | 是 | 文件上传、下载、删除、解析、OCR 和版面分析 |
| `images` | 是 | 图像生成 |
| `mcp` | 否 | 基于 `rmcp` 的独立 Streamable HTTP MCP 客户端 |
| `rag` | 是 | Retrieval Agent、知识库与文档管理 |
| `realtime` | 是 | 强类型双向 WebSocket 客户端 |
| `tools` | 是 | 托管工具类型、Web 操作和 ToolStream 聚合 |
| `video` | 是 | 视频生成 |
| `full` | 否 | 启用包括 `mcp` 在内的全部 feature |

最小 HTTP 聊天客户端：

```toml
[dependencies]
rustglm = { version = "1.0.0", default-features = false }
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

选择部分企业 API：

```toml
[dependencies]
rustglm = {
    version = "1.0.0",
    default-features = false,
    features = ["batch", "mcp", "rag", "realtime", "tools"]
}
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

全部 API：

```toml
[dependencies]
rustglm = { version = "1.0.0", features = ["full"] }
```

## 认证

`ZhipuClient::new`、`ZhipuConfig::new` 和 `RealtimeConfig::new` 均直接接收凭据。

- `key_id.secret` 被视为智谱组合 API Key，并签名为 HS256 JWT。
- 任何其他非空值均被视为不透明 Bearer token。
- 自动选择不合适时，可使用 `ZhipuAuthentication::jwt` 或 `ZhipuAuthentication::bearer`。

请勿提交凭据。应用应自行从进程环境、密钥管理器或工作负载身份提供方读取密钥。

## 强类型 GLM-5 聊天

标记类型、封闭能力 trait 和请求 typestate 会阻止通过强类型 API 发送不支持的操作。请求在包含用户或工具输入前，无法传给强类型补全方法。

```rust,no_run
use rustglm::{Glm52, ReasoningEffort, Thinking, TypedChatRequest, ZhipuClient};

# async fn run() -> rustglm::Result<()> {
let client = ZhipuClient::new("key_id.secret")?;
let request = TypedChatRequest::<Glm52>::new()
    .system("Answer with evidence.")
    .thinking(Thinking::enabled())
    .reasoning_effort(ReasoningEffort::High)
    .user("Summarize the incident report.");
let response = client.typed_chat_completion(&request).await?;
println!("{}", response.text().unwrap_or_default());
# Ok(())
# }
```

### 支持的聊天模型

下表描述编译期强类型 API。新发布或私有模型 ID 仍可通过原始 `ChatCompletionRequest` 使用。

| 文本模型 | 标记类型 | Thinking | Reasoning effort | ToolStream |
| --- | --- | :---: | :---: | :---: |
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

`ReasoningEffort`、`Thinking`、ToolStream、工具和视觉输入只会暴露给声明相应能力的标记类型，从而阻止不受支持的字段通过强类型 API 到达传输层。

当新发布字段尚未获得强类型 builder 时，`ChatCompletionRequest` 仍可作为前向兼容的原始请求使用。

## ToolStream

ToolStream 将碎片化 SSE 函数调用增量合并为完整的强类型调用，同时保留文本、推理、用量和流错误。

```rust,no_run
use futures_util::StreamExt;
use rustglm::{Glm52, ToolStreamEvent, TypedChatRequest, ZhipuClient};

# async fn run() -> rustglm::Result<()> {
let client = ZhipuClient::new("token")?;
let request = TypedChatRequest::<Glm52>::new().tool_stream().user("Check the deployment status.");
let mut stream = client.typed_chat_tool_stream(&request).await?;
while let Some(event) = stream.next().await {
    if let ToolStreamEvent::ToolCallCompleted(call) = event? {
        println!("{} {}", call.name, call.arguments);
    }
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

可用方法为 `create_batch`、`batches`、`batch` 和 `cancel_batch`。不在 `1..=100` 范围内的列表限制会在网络 I/O 前返回 `BatchError::InvalidLimit`。

## 知识库与 RAG

`rag` feature 遵循官方知识库 OpenAPI 路径，涵盖知识库 CRUD、容量、检索、文档列表和详情、内存文件上传、URL 摄取、删除、文档图片和重新嵌入。`RagDocumentUpload::from_bytes` 有意不提供基于路径的构造函数；调用方控制文件读取、大小限制、加密、租户边界和保留策略。

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

## MCP 客户端

`mcp` feature 是独立的 Model Context Protocol 客户端，与在模型请求中配置托管 MCP 工具的 `McpTool` 不同。协议帧、初始化、工具、资源、提示词和 Streamable HTTP 传输由官方 Rust MCP SDK（`rmcp`）提供。

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

安全默认值：仅接受绝对 `http` 和 `https` 端点；授权显式配置并从 `Debug` 输出中脱敏；SDK 创建的 HTTP 客户端禁用重定向；SSE 重试和过期会话自动初始化默认关闭；可注入调用方配置的 `reqwest::Client` 控制代理、TLS、DNS、超时和策略。

## Realtime WebSocket

`realtime` feature 通过双向 WebSocket 提供强类型客户端请求和服务器事件。音视频作为调用方拥有的字节切片传入，并在内存中编码。

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

该 API 还支持强类型会话工具、函数调用输出、响应选项、转录会话、客户端/服务端 VAD、取消、音频提交/清空、视频帧和显式关闭连接。

## 错误

`SdkError` 是公共错误封装。领域错误是显式枚举，可直接匹配，无需解析展示字符串。

```rust
use rustglm::{BatchError, SdkError};

fn classify(error: SdkError) {
    match error {
        SdkError::Batch(BatchError::InvalidLimit(limit)) => eprintln!("invalid batch limit: {limit}"),
        SdkError::Api(api) => eprintln!("HTTP {} request_id={:?}", api.status, api.request_id),
        other => eprintln!("{other}"),
    }
}
```

该封装区分配置、校验、传输、超时、API、解码、流、WebSocket、不支持能力、Agent、工具、Batch、RAG 和 MCP 失败。`ApiError` 保留 HTTP 状态、厂商代码、消息、请求 ID 和原始响应体。

## HTTP 策略

`HttpConfig` 控制请求超时、连接超时、连接池空闲超时、user agent、默认请求头、重试策略，以及可选的调用方构建 `reqwest::Client`。

重试默认关闭。启用 `RetryPolicy` 是应用的显式决定；只有配置的状态码及连接/超时失败会被重试。

## API 覆盖范围

下表是公开 SDK 操作的索引，依据公开客户端接口整理，而非假定服务商能力。接受 `serde_json::Value` 的方法有意保留与快速变化的服务商 Schema 的兼容性。

| 能力领域 | Feature | 公开方法 |
| --- | --- | --- |
| 聊天与流 | 核心；ToolStream 需 `tools` | `chat_completion`、`chat_completion_stream`、`chat_tool_stream`、`typed_chat_completion`、`typed_chat_completion_stream`、`typed_chat_tool_stream` |
| 异步与向量 API | 核心 | `async_chat`、`async_result`、`embedding`、`rerank`、`tokenizer` |
| 图像与视频 | `images`、`video` | `create_image`、`create_image_async`、`create_video` |
| 音频与音色 | `audio` | `glm_4_voice`、`transcribe`、`speech`、`clone_voice`、`voices`、`delete_voice` |
| 托管工具 | `tools` | `web_search`、`read_web_page`、`moderate` |
| 文件与文档处理 | `files` | `upload_file`、`files`、`file_content`、`delete_file`、`create_file_parse_task`、`file_parse_result`、`parse_file_sync`、`ocr`、`parse_layout` |
| Batch | `batch` | `create_batch`、`batches`、`batch`、`cancel_batch` |
| 官方 Agent 与 Assistant | `agents` | `official_agent`、`official_agent_stream`、`official_agent_async_result`、`official_agent_conversation`、`assistant`、`assistants`、`assistant_conversations` |
| 知识库与检索 | `rag` | `create_knowledge_base`、`knowledge_bases`、`knowledge_base`、`update_knowledge_base`、`delete_knowledge_base`、`knowledge_capacity`、`retrieve_knowledge`、`knowledge_documents`、`upload_knowledge_document`、`upload_knowledge_urls`、`knowledge_document`、`delete_knowledge_document`、`knowledge_document_images`、`reembed_knowledge_document`、`retrieval_agent_stream` |
| 通用协议入口 | 核心 | `ZhipuClient` 与 `OpenAiCompatibleClient` 上的 `request_json` |
| 独立 MCP | `mcp` | `McpClientConfig::connect`，以及由 `rmcp` 提供的强类型工具、资源、提示词和 Streamable HTTP 操作 |
| Realtime | `realtime` | `RealtimeConfig::connect`、强类型请求/事件、VAD、媒体缓冲、函数调用输出、取消与显式关闭 |

服务商已发布字段尚未获得强类型 builder 时，使用 `ChatCompletionRequest`。只有在配置好的服务商 Base URL 下需要新相对路径时才使用 `request_json`；它会拒绝绝对 URL 与父级路径段。

> RustGLM 则提供服务商无关的本地 Agent 运行时、OpenAI 兼容客户端、通用 `rmcp` 协议客户端和支持视频的 Realtime 会话。

## 示例

仓库包含 36 个可运行示例。当前每个 HTTP 端点领域都有聚焦示例；通常一起使用的操作会放进同一个生命周期示例。`cargo check --all-targets --all-features` 可在不联系服务商的情况下编译检查全部示例。

### 聊天、模型与向量

| 示例 | 演示的公开 API |
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

| 示例 | 演示的公开 API |
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

| 示例 | 演示的公开 API |
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

| 示例 | 演示的公开 API |
| --- | --- |
| [`official_agent`](examples/official_agent.rs) | 强类型官方 Agent v1 调用 |
| [`official_agent_lifecycle`](examples/official_agent_lifecycle.rs) | Agent 流、异步结果与会话操作 |
| [`assistants`](examples/assistants.rs) | Assistant 调用、列表与会话 |
| [`custom_agent`](examples/custom_agent.rs) | 带应用工具的本地 Agent 运行时 |
| [`interactive_chat`](examples/interactive_chat.rs) | 多轮运行时与可选语义记忆 |
| [`mcp_client`](examples/mcp_client.rs) | MCP 工具、资源、提示词与关闭连接 |
| [`realtime_audio_video`](examples/realtime_audio_video.rs) | Realtime PCM/WAV、可选 JPEG 帧与强类型事件 |

通过 `cargo run --example <name> -- <参数>` 运行示例。MCP 客户端为按需 feature，请使用 `cargo run --example mcp_client --features mcp -- <endpoint>`。大多数智谱示例需要 `ZHIPU_API_KEY`；`openai_compatible` 使用 `OPENAI_COMPATIBLE_BASE_URL` 与 `OPENAI_COMPATIBLE_API_KEY`。运行示例可能消耗额度、创建远程资源，或删除命令行中明确指定的资源。

## CI 与发布

CI 工作流会验证：

- 格式化；
- 将警告视为错误的 Clippy；
- 无默认 feature、默认 feature、全部 feature 和单独企业 feature 构建；
- 测试和 doctest；
- 将警告视为错误的文档构建；
- 从已提交 lockfile 构建包；
- 全 feature 行覆盖率不低于 90%，并上传 LCOV 与文本摘要。

发布工作流会在 `v*` tag 推送时运行。手动运行时，请在 Actions 页面选择要发布的提交或分支，并在 `tag` 输入中填写 `v<Cargo.toml version>`。工作流会检出页面所选版本，不再假定 tag 已经存在；它会拒绝版本不匹配，执行全部发布门禁，构建 `.crate`、写入 `SHA256SUMS`，然后创建缺失的附注 tag。已有 tag 只有在指向本次验证的提交时才会被接受。最后，工作流会在配置 `CARGO_REGISTRY_TOKEN` 时可选发布到 crates.io，并创建或更新 GitHub Release。

发布步骤：

```bash
# 请先更新 Cargo.toml 与发布说明；Cargo.toml 当前版本为 1.0.0。
git tag -s v1.0.0 -m "RustGLM v1.0.0"
git push origin v1.0.0
```

也可以在 `main` 分支上手动运行 `Release` 工作流，并将 `tag` 填为 `v1.0.0`，无需预先创建 tag。tag 使用普通的 `v1.0.0` 格式，而不是 `RustGLM v1.0.0`。

仓库中不保存 API Key 或 registry token。仅在需要发布 crates.io 时，将 `CARGO_REGISTRY_TOKEN` 配置为 GitHub Actions secret。

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

仓库提供统一覆盖率命令，并由 CI 强制执行最低门槛：

```bash
cargo coverage       # 输出摘要并检查 90% 行覆盖率门槛
cargo coverage-lcov  # 生成 target/rustglm-lcov.info，并检查相同门槛
```

最近一次工作区实测快照（2026-07-24）：

| 测试 | Regions | Functions | Lines | 行覆盖率门槛 |
| ---: | ---: | ---: | ---: | ---: |
| 94 个通过、2 个真实服务测试忽略 | 92.72% | 88.33% | 94.02% | 90.00% |

全 feature 测量包含所有库模块，包括可选的 MCP 与 Realtime。知识库/RAG 行覆盖率为 96.63%；成功的 MCP 协议操作需要已初始化的对端，因此其离线行覆盖率为 51.60%。完整模块表、指标解释与 HTML 报告命令见 [docs/COVERAGE.md](docs/COVERAGE.md)。

覆盖率命令会运行离线单元测试与集成测试，并编译全部 36 个示例，但不会执行示例的 `main` 函数，也不会运行被忽略的真实服务测试。请显式运行需要凭据的检查：

```powershell
$env:ZHIPU_API_KEY = "key_id.secret"
cargo test --test live_zhipu -- --ignored --nocapture
cargo test --test live_realtime -- --ignored --nocapture
```

CI 会重新生成数据，并将 `lcov.info` 与 `coverage-summary.txt` 发布为构建产物；评估具体提交时应以该产物为准，不应把上面的快照视为永久承诺。

## 许可证

Apache License 2.0。参见 [LICENSE](LICENSE)。
