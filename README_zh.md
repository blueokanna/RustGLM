# [RustGLM](README.md) - 智谱 AI 开放平台 Rust SDK

[![CI](https://github.com/blueokanna/rustglm/actions/workflows/ci.yml/badge.svg)](https://github.com/blueokanna/rustglm/actions/workflows/ci.yml)
[![Release](https://github.com/blueokanna/rustglm/actions/workflows/release.yml/badge.svg)](https://github.com/blueokanna/rustglm/actions/workflows/release.yml)
[![Crates.io](https://img.shields.io/crates/v/rustglm.svg)](https://crates.io/crates/rustglm)
[![Documentation](https://docs.rs/rustglm/badge.svg)](https://docs.rs/rustglm)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)

RustGLM 是面向智谱 AI 开放平台的非官方异步 Rust SDK，提供强类型 GLM-5 请求、SSE 和 ToolStream 聚合、双向 Realtime WebSocket 会话、Batch API 操作、知识库管理，以及基于官方 Rust MCP SDK 的 MCP 客户端。

本 crate 面向生产后端。网络策略、持久化、凭据、重试和客户端生命周期均由应用显式控制。

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

当前强类型模型标记包括：

- 文本：`Glm52`、`Glm51`、`Glm5Turbo`、`Glm5`、`Glm47`、GLM-4.7 Flash 变体、GLM-4.6，以及部分 GLM-4.5/4 Flash 模型。
- 视觉：`Glm5vTurbo`、GLM-4.6V 变体、`Glm4vFlash` 和 GLM-4.1V Thinking 变体。
- `ReasoningEffort` 仅对 `Glm52` 开放。
- `Thinking`、工具、ToolStream 和视觉输入只会暴露给声明这些能力的标记类型。

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

## CI 与发布

CI 工作流会验证：

- 格式化；
- 将警告视为错误的 Clippy；
- 无默认 feature、默认 feature、全部 feature 和单独企业 feature 构建；
- 测试和 doctest；
- 将警告视为错误的文档构建；
- 从已提交 lockfile 构建包。

发布工作流会在 `v*` tag 推送时运行。手动运行时，请在 `tag` 输入中填写已存在的 `v*` tag。工作流会检出该 tag，拒绝与 `v<Cargo.toml version>` 不完全一致的 tag，重新执行发布门禁，构建 `.crate`、写入 `SHA256SUMS`，在配置 `CARGO_REGISTRY_TOKEN` 时可选发布到 crates.io，并使用这些产物创建 GitHub Release。

发布步骤：

```bash
# Update Cargo.toml and changelog/release notes first.
git tag -s v0.2.1 -m "RustGLM v0.2.1"
git push origin v0.2.1
```

仓库中不保存 API Key 或 registry token。仅在需要发布 crates.io 时，将 `CARGO_REGISTRY_TOKEN` 配置为 GitHub Actions secret。

## 本地测试

```bash
cargo fmt --all -- --check
cargo test --all-targets --no-default-features
cargo test --all-targets
cargo test --all-targets --all-features
cargo clippy --all-targets --all-features -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --all-features --no-deps
cargo package --locked
```

实时测试默认被忽略，因为它们需要凭据且可能产生费用。

## 许可证

Apache License 2.0。参见 [LICENSE](LICENSE)。
