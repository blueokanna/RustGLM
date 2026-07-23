# [RustGLM](README.md) - Rust SDK for Zhipu AI Open Platform

面向智谱 AI 开放平台的非官方生产级 Rust SDK，同时提供可扩展的多厂商 LLM Provider 架构。

当前版本：`1.0.0`
当前许可证：`Apache-2.0`
Rust 版本 1.88 或更高


## 定位

RustGLM 是 SDK，不是命令行聊天程序，也不是把一次 HTTP/HTTPS 调用包装成函数的示例项目。

基础客户端不会隐式保存聊天记录、读取固定 TOML 配置、解析特殊分隔符、访问 NTP、输出日志或选择模型。只有调用者显式创建的 `Conversation`、`AgentRuntime` 和记忆实现才持有实例级状态；应用仍负责持久化、租户隔离和生命周期管理。

当前智谱协议依据官方 OpenAPI，模型 API 与 Agent API 默认地址分别为：

```text
https://open.bigmodel.cn/api/paas/v4
https://open.bigmodel.cn/api
```

默认 feature 包含原生 GLM-Realtime。只需要 HTTP、Agent、记忆和 GLM-4-Voice 时可关闭它以减少依赖：

```toml
[dependencies]
RustGLM = { version = "1.0.0", default-features = false }
```

认证使用标准 Bearer 请求头。传入智谱组合 API Key 时，SDK 会按照官方 Python、Java 和 Realtime SDK 的规则生成 JWT：

```text
Authorization: Bearer <JWT_OR_OPAQUE_TOKEN>
```

## 能力

- 文本对话
- SSE 流式对话
- GLM-Realtime 实时音频、视频和文本通话
- GLM-4-Voice 端到端语音输入、音频输出与 PCM/WAV 转换
- Client VAD、Server VAD、实时打断和 Function Calling
- 图片、视频、音频和文件理解
- 深度思考与推理强度
- Function Calling
- 智谱 Web Search、Retrieval、MCP 等可配置工具
- 同步和异步聊天
- 同步和异步图像生成
- 视频生成和异步任务查询
- Embedding
- Rerank
- Tokenizer
- 语音转文字
- 文字转语音
- 音色复刻、列表和删除
- 文件上传、列表、下载和删除
- 文档解析、OCR 和版面解析
- Web Search、网页阅读和内容安全
- Batch
- Assistant
- 智谱官方 v1 Agent 同步、SSE、异步结果和对话历史
- 智谱知识问答 ReAct Agent SSE
- 可定制角色、工具执行循环、步数保护和执行轨迹
- 不含密钥的跨设备 Agent 部署清单与可替换密钥解析器
- 全部官方 v4 端点的泛型请求接口
- 可配置连接、超时、连接池、默认请求头和重试
- 结构化 API 错误、请求 ID 和原始响应
- 可替换 `reqwest::Client`
- 厂商无关 `ChatProvider`
- OpenAI-compatible 服务适配
- 无状态、最近消息和语义向量记忆模式
- 可插拔 Embedding、ConversationMemory 和 VectorStore
- int8 量化向量压缩、余弦召回和可序列化快照

## 安装

```toml
[dependencies]
rustglm = "1.0.0"
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
futures-util = "0.3"
serde_json = "1"
```

## API Key

不要将 API Key 写入源码或提交到仓库。

PowerShell：

```powershell
$env:ZHIPU_API_KEY="your-api-key"
```

Bash：

```bash
export ZHIPU_API_KEY="your-api-key"
```

## JWT 认证

`ZhipuClient::new`、`ZhipuConfig::new` 和 `RealtimeConfig::new` 默认自动选择认证模式。`key_id.secret` 会生成 JWT；不含点号的非空值作为已有 Bearer token 使用。格式错误的组合 Key 会直接返回 `SdkError::Configuration`，不会静默降级。

JWT 格式与智谱官方 SDK 保持一致：

- Header 为 `{"alg":"HS256","sign_type":"SIGN"}`
- Payload 包含 `api_key`、Unix 毫秒 `timestamp` 和 Unix 毫秒 `exp`
- 使用 API secret 执行 HMAC-SHA256 签名
- 默认有效期 180 秒，提前 30 秒刷新
- Base64 使用 URL-safe、无 padding 编码
- HTTP 与 Realtime WebSocket 均发送 `Authorization: Bearer <token>`

显式 JWT 配置：

```rust
use std::time::Duration;

use rustglm::{JwtAuthentication, ZhipuAuthentication, ZhipuConfig};

let jwt = JwtAuthentication::from_api_key(std::env::var("ZHIPU_API_KEY")?)?
    .token_ttl(Duration::from_secs(180))
    .refresh_before(Duration::from_secs(30))
    .cache_enabled(true);

let client = ZhipuConfig::new("unused")
    .authentication(ZhipuAuthentication::jwt(jwt))
    .build()?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

如果上层网关已经签发完整 Bearer token，可使用 `ZhipuAuthentication::bearer`。JWT 缓存在可克隆客户端共享的认证状态中，SDK 使用本机 `SystemTime`，不访问 NTP。

## 最小调用

```rust
use rustglm::{ChatCompletionRequest, ChatMessage, ZhipuClient};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = ZhipuClient::new(std::env::var("ZHIPU_API_KEY")?)?;
    let request = ChatCompletionRequest::new("glm-5.2")
        .message(ChatMessage::system("你是一个严谨的 Rust 工程师"))
        .message(ChatMessage::user("解释 Pin 和 Unpin 的区别"));
    let response = client.chat_completion(&request).await?;
    println!("{}", response.text().unwrap_or_default());
    Ok(())
}
```

客户端可安全克隆。内部连接池由 `reqwest::Client` 共享，建议为每个配置创建一个长生命周期客户端，不要为每次请求重新创建。

## 交互式测试

文本交互示例会提示输入 API Key、模型、角色名称、角色定位、表达风格和上下文模式：

```bash
cargo run --example interactive_chat
```

上下文模式：

- `0`：完全无状态，每轮只发送系统提示和当前问题
- `1`：保留最近 20 条消息
- `2`：使用 `embedding-3` 生成向量、int8 压缩保存到 JSON 文件并按相似度召回

输入 `clear` 可清空当前历史、语义记忆及对应快照文件，输入 `exit` 或 `quit` 退出。模式 `2` 会提示记忆文件路径，默认使用 `rustglm-memory.json`，启动时恢复并在每轮成功后保存。该文件包含明文对话，模式 `2` 会额外调用 Embedding API，可能产生费用。

如果未设置环境变量，示例允许直接输入 Key，但输入内容会显示。推荐先设置临时环境变量：

```powershell
$env:ZHIPU_API_KEY="your-key-id.your-secret"
cargo run --example interactive_chat
```

## 上下文与语义记忆

基础 `ZhipuClient` 和 `ChatProvider` 不会隐式保存上下文。需要状态管理时使用 `Conversation`，是否保留上下文由调用者明确配置。

### 完全无状态

```rust
use std::sync::Arc;
use rustglm::{Conversation, ConversationConfig, ZhipuClient};

let client = ZhipuClient::new(std::env::var("ZHIPU_API_KEY")?)?;
let mut conversation = Conversation::new(
    Arc::new(client),
    ConversationConfig::new("glm-5.2")
        .system_prompt("你是一个严谨的助手"),
)?;

let response = conversation.send("这轮不会记住之前的消息").await?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

### 最近消息上下文

```rust
use std::sync::Arc;
use rustglm::{Conversation, ConversationConfig, ZhipuClient};

let client = ZhipuClient::new(std::env::var("ZHIPU_API_KEY")?)?;
let mut conversation = Conversation::new(
    Arc::new(client),
    ConversationConfig::new("glm-5.2")
        .retain_history(true)
        .max_history_messages(20),
)?;

conversation.send("记住我的项目使用 Rust").await?;
let response = conversation.send("我的项目使用什么语言？").await?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

### 语义向量记忆

```rust
use std::sync::Arc;
use rustglm::{
    Conversation, ConversationConfig, InMemoryVectorStore, SemanticMemory,
    ZhipuClient, ZhipuEmbeddingProvider,
};

let client = ZhipuClient::new(std::env::var("ZHIPU_API_KEY")?)?;
let embeddings = Arc::new(ZhipuEmbeddingProvider::new(
    client.clone(),
    "embedding-3",
));
let store = Arc::new(InMemoryVectorStore::new());
let memory = Arc::new(SemanticMemory::new(embeddings, store.clone()));
let mut conversation = Conversation::new(
    Arc::new(client),
    ConversationConfig::new("glm-5.2")
        .semantic_memory(memory, 4),
)?;

conversation.send("RustGLM 的部署区域是上海").await?;
let response = conversation.send("项目部署在哪里？").await?;
let snapshot = store.snapshot_json()?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

`QuantizedVector` 将 `f32` embedding 按向量独立缩放为 int8，向量主体从每维 4 字节降到每维 1 字节，代价是轻微量化误差。`InMemoryVectorStore` 支持 upsert、余弦搜索、清空、JSON 快照和恢复。快照同时包含对话文本，持久化前应按业务要求加密并控制访问。

扩展接口：

- 实现 `EmbeddingProvider` 可接入 OpenAI、Qwen、本地 embedding 或批量服务
- 实现 `VectorStore` 可接入 Qdrant、Milvus、pgvector、Redis 或业务数据库
- 实现 `ConversationMemory` 可替换召回、摘要、分层记忆或自定义压缩策略
- `MemoryDocument.metadata` 可保存租户、会话、时间和权限标签
- `Conversation::clear_history` 与 `Conversation::clear_memory` 分别控制短期和语义记忆

SDK 不把本地伪随机数当作 embedding。内置 `ZhipuEmbeddingProvider` 会真正调用智谱 Embedding API；测试使用显式假的 `EmbeddingProvider`，不会把测试向量伪装成生产向量。

## 自定义智能体

`AgentRuntime` 构建在厂商无关的 `ChatProvider` 上，负责角色系统提示词、工具调用循环、最大模型步数、最近消息和可选语义记忆。它不会绕过服务商规则，也不会自动执行任意系统命令；只有显式注册的 `AgentTool` 才能执行。

```rust
use std::sync::Arc;
use rustglm::{
    AgentHistoryPolicy, AgentManifest, AgentPersona, AgentRuntime, ZhipuClient,
};

let persona = AgentPersona::new("洛书", "有鲜明风格的 Rust 技术伙伴")
    .background("长期维护跨平台异步系统")
    .trait_value("直接")
    .trait_value("严谨")
    .speaking_style("先给结论，再给依据")
    .language("简体中文")
    .instruction("不知道时明确说明，不伪造来源")
    .boundary("不要假装已经执行未注册的工具");
let manifest = AgentManifest::new("glm-5.2", persona)
    .history(AgentHistoryPolicy::Recent { max_messages: 20 });
let client = ZhipuClient::new(std::env::var("ZHIPU_API_KEY")?)?;
let mut agent = AgentRuntime::new(Arc::new(client), manifest)?;
let result = agent.run("分析这个 Rust 服务的部署风险").await?;
println!("{}", result.response.text().unwrap_or_default());
# Ok::<(), Box<dyn std::error::Error>>(())
```

`AgentTool` 使用 JSON Schema 向模型公开参数，使用结构化 `Value` 接收参数与返回结果。运行时会拒绝重复工具名、未知工具、缺失函数负载和非法 JSON 参数；`AgentRunResult` 返回模型调用步数与每次工具执行的调用 ID、工具名、参数和输出。超过 `AgentManifest.max_steps` 时返回 `SdkError::Agent`，避免无限工具循环。

完整工具调用示例：

```bash
cargo run --example custom_agent
```

角色配置是开放的数据模型，`background`、`traits`、`speaking_style`、`language`、`instructions` 和 `boundaries` 均由应用控制。SDK 不硬编码单一人设，但服务商内容规则、当地法律和应用自己的安全边界仍然生效。

### 智谱官方智能体

官方 `/api/v1/agents` 协议使用独立于 v4 模型 API 的基础地址，并与模型客户端共享认证和 JWT 缓存。已提供强类型多模态消息、输出、异步状态和使用量：

```rust
use rustglm::{
    OfficialAgentMessage, OfficialAgentRequest, TranslationAgentVariables, ZhipuClient,
};

let client = ZhipuClient::new(std::env::var("ZHIPU_API_KEY")?)?;
let request = OfficialAgentRequest::new("general_translation")
    .message(OfficialAgentMessage::user("Translate: memory safety"))
    .custom_variables(TranslationAgentVariables {
        source_lang: "en".into(),
        target_lang: "zh-CN".into(),
        ..Default::default()
    })?;
let response = client.official_agent(&request).await?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

对应方法：

| 方法 | 官方端点 | 返回方式 |
| --- | --- | --- |
| `official_agent` | `POST /api/v1/agents` | JSON |
| `official_agent_stream` | `POST /api/v1/agents` | SSE |
| `official_agent_async_result` | `POST /api/v1/agents/async-result` | JSON |
| `official_agent_conversation` | `POST /api/v1/agents/conversation` | JSON |
| `retrieval_agent_stream` | `POST /api/zrag/agent/chat` | ReAct SSE |

`OfficialAgentInputPart` 支持 `text`、`file_id`、`file_url` 和 `image_url`。官方输出支持文本、文件、图片、音频和视频 URL。`RetrievalAgentRequest` 对知识库 ID、召回数量、重排、相似度阈值、思考模式和最大推理步数建模，续聊通过 `X-Session-Id` 请求头发送。

真实官方智能体测试入口：

```bash
cargo run --example official_agent
```

### 跨设备部署

`AgentManifest` 可序列化为 JSON，包含协议、Base URL、模型、人设、温度、步数和历史策略，但只保存 `CredentialReference`，绝不保存 API Key 或 JWT。默认引用 `ZHIPU_API_KEY`：

```rust
use rustglm::{AgentManifest, AgentRuntime, EnvironmentSecretResolver};

let json = std::fs::read_to_string("agent.json")?;
let manifest = AgentManifest::from_json(&json)?;
let runtime = AgentRuntime::from_manifest(manifest, &EnvironmentSecretResolver)?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

实现 `SecretResolver` 可接入 Kubernetes Secret、Docker Secret、系统钥匙串、移动端安全存储或云密钥管理服务。实现 `ChatProvider` 可复用相同 Agent 运行时接入新协议。HTTP 使用 Rustls，避免依赖目标系统的 OpenSSL；桌面和服务器使用 Tokio 即可部署。GLM-Realtime 依赖原生 TCP/WebSocket，浏览器 WASM 目标不能直接复用该模块，移动端麦克风和摄像头采集也应由平台层实现后把字节传给 SDK。

## 自定义客户端

```rust
use std::time::Duration;

use reqwest::header::{HeaderMap, HeaderValue};
use rustglm::{HttpConfig, RetryPolicy, ZhipuConfig};

fn build_client() -> rustglm::Result<rustglm::ZhipuClient> {
    let mut headers = HeaderMap::new();
    headers.insert("x-application-name", HeaderValue::from_static("my-service"));

    let http = HttpConfig {
        timeout: Duration::from_secs(180),
        connect_timeout: Duration::from_secs(8),
        pool_idle_timeout: Duration::from_secs(90),
        user_agent: "my-service/1.0".into(),
        default_headers: headers,
        retry: RetryPolicy {
            max_retries: 2,
            initial_delay: Duration::from_millis(300),
            max_delay: Duration::from_secs(3),
            ..RetryPolicy::default()
        },
        http_client: None,
    };

    ZhipuConfig::new(std::env::var("ZHIPU_API_KEY").unwrap_or_default())
        .http(http)
        .build()
}
```

也可以通过 `HttpConfig::http_client` 注入已有的 `reqwest::Client`，统一使用应用的代理、TLS、DNS、连接池和观测配置。

重试默认关闭。生成类 POST 请求可能产生重复任务或重复计费，只有在业务具有 `request_id` 幂等策略并能够接受风险时才应开启自动重试。流开始后不会重试。

## 对话参数

```rust
use rustglm::{
    ChatCompletionRequest, ChatMessage, ReasoningEffort, ResponseFormat,
    ResponseFormatType, Thinking,
};

let mut request = ChatCompletionRequest::new("glm-5.2")
    .message(ChatMessage::user("输出一个包含 name 和 version 的 JSON 对象"))
    .temperature(0.2)
    .max_tokens(2048)
    .thinking(Thinking::enabled())
    .request_id("request-20260723-000001");

request.reasoning_effort = Some(ReasoningEffort::High);
request.response_format = Some(ResponseFormat {
    kind: ResponseFormatType::JsonObject,
});
```

`temperature`、`top_p`、`max_tokens`、`thinking` 和 `reasoning_effort` 的有效范围与实际能力由所选模型决定。SDK 会拒绝明显非法的通用范围，但不会把模型名称或模型专属上限固化在客户端中。

## 流式输出

```rust
use futures_util::StreamExt;
use rustglm::{ChatCompletionRequest, ChatMessage, ResponseContent, ZhipuClient};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = ZhipuClient::new(std::env::var("ZHIPU_API_KEY")?)?;
    let request = ChatCompletionRequest::new("glm-5.2")
        .message(ChatMessage::user("写一个无锁队列的设计说明"));
    let mut stream = client.chat_completion_stream(&request).await?;

    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        for choice in chunk.choices {
            if let Some(ResponseContent::Text(text)) = choice.delta.content {
                print!("{text}");
            }
        }
    }

    Ok(())
}
```

SSE 解码器按字节缓冲，能够处理 JSON、UTF-8 字符和事件边界被拆分到不同网络数据块的情况，并识别官方 `[DONE]` 结束事件。

## GLM-Realtime 音视频通话

Realtime 客户端对接官方 WebSocket 地址：

```text
wss://open.bigmodel.cn/api/paas/v4/realtime
```

SDK 提供持续连接、自动 JWT/Bearer 鉴权、并发收发通道、WebSocket Ping/Pong、JSON 事件编解码和未知服务端字段保留。它负责协议和媒体数据传输，不会隐式打开麦克风、摄像头或扬声器；设备权限、采集、JPEG 编码和播放由应用层决定。

Client VAD 音视频调用：

```rust
use rustglm::{RealtimeClient, RealtimeSession};

let mut connection = RealtimeClient::connect(
    std::env::var("ZHIPU_API_KEY")?
).await?;

let sender = connection.sender();
sender.update_session(
    RealtimeSession::default()
        .model("glm-realtime-flash")
        .instructions("请用简洁自然的中文回答")
        .input_audio_format("pcm16")
        .video(),
).await?;

sender.append_audio(&pcm_16khz_mono_bytes).await?;
sender.append_video_frame(&jpeg_frame_bytes).await?;
sender.commit().await?;
sender.create_response().await?;

while let Some(event) = connection.next_event().await {
    let event = event?;
    if let Some(text) = event.delta_text() {
        print!("{text}");
    }
    if let Some(pcm) = event.audio_bytes()? {
        play_or_store_pcm(pcm);
    }
    if event.event_type == "response.done" {
        break;
    }
}
# Ok::<(), Box<dyn std::error::Error>>(())
```

上例中的 `pcm_16khz_mono_bytes`、`jpeg_frame_bytes` 和 `play_or_store_pcm` 由应用提供。需要同时采集和播放时，可调用 `connection.split()` 得到可克隆的 `RealtimeSender` 和独立的 `RealtimeReceiver`，分别放入 Tokio task。

Server VAD：

```rust
use rustglm::RealtimeSession;

let session = RealtimeSession::default()
    .model("glm-realtime-air")
    .server_vad(true, true);
```

Server VAD 会自动检测开始和结束，并可自动创建响应和打断当前回复。Client VAD 需要显式调用 `commit` 和 `create_response`。`cancel_response` 可主动打断输出。

官方媒体约束：

- 输入支持 WAV 或 PCM；PCM 仅支持单声道、16 位深，`pcm16` 表示 16kHz、`pcm24` 表示 24kHz
- 输出当前为 24kHz、单声道、16 位 PCM
- 视频帧为 Base64 JPEG，视频模式使用 `video_passive`
- 单次音频最长 30 秒，最高发送速率 50 QPS，官方建议按 100ms 一帧、每秒 10 帧
- `glm-realtime-flash` 和 `glm-realtime-air` 的权限、价格和可用范围以账号控制台为准

已建模的客户端事件包括：

- `session.update` 与 `transcription_session.update`
- `input_audio_buffer.append`、`append_video_frame`、`commit` 和 `clear`
- `conversation.item.create`、`delete` 和 `retrieve`
- `response.create` 与 `response.cancel`

服务端事件使用 `RealtimeServerEvent` 接收。`delta_text()` 同时处理文本和音频转写增量，`audio_bytes()` 解码音频，`function_call()` 提取函数名和 JSON 参数，`error()` 提取官方错误对象；原始新增字段保存在 `data` 中，因此官方增加事件字段时不会导致反序列化失败。

仓库中的文件驱动示例可验证真实音视频链路：

```bash
cargo run --example realtime_audio_video
```

它读取 PCM16/WAV 音频和可选 JPEG 帧，按 100ms 音频块发送，并将模型返回音频保存为 `realtime-output.pcm`。这会访问真实服务并可能产生费用。

## 多模态

### 图片

```rust
use rustglm::{ChatCompletionRequest, ChatMessage, ContentPart, MessageRole};

let request = ChatCompletionRequest::new("glm-5v-turbo").message(
    ChatMessage::multimodal(
        MessageRole::User,
        vec![
            ContentPart::image_url("https://example.com/image.png"),
            ContentPart::text("说明图片中的内容"),
        ],
    ),
);
```

### 视频

```rust
use rustglm::{ChatCompletionRequest, ChatMessage, ContentPart, MessageRole};

let request = ChatCompletionRequest::new("glm-5v-turbo").message(
    ChatMessage::multimodal(
        MessageRole::User,
        vec![
            ContentPart::video_url("https://example.com/video.mp4"),
            ContentPart::text("总结视频内容"),
        ],
    ),
);
```

### 文件

```rust
use rustglm::{ChatCompletionRequest, ChatMessage, ContentPart, MessageRole};

let request = ChatCompletionRequest::new("glm-5v-turbo").message(
    ChatMessage::multimodal(
        MessageRole::User,
        vec![
            ContentPart::file_url("https://example.com/report.pdf"),
            ContentPart::text("提取报告中的风险项"),
        ],
    ),
);
```

### GLM-4-Voice

```rust
use rustglm::{Glm4VoiceRequest, ZhipuClient};

let client = ZhipuClient::new(std::env::var("ZHIPU_API_KEY")?)?;
let input = std::fs::read("input.wav")?;
let request = Glm4VoiceRequest::from_wav("请慢速复述", &input)?;
let response = client.glm_4_voice(&request).await?;
println!("{}", response.text().unwrap_or_default());
if let Some(wav) = response.audio_wav()? {
    std::fs::write("output.wav", wav)?;
}
# Ok::<(), Box<dyn std::error::Error>>(())
```

官方 GLM-4-Voice 使用 `glm-4-voice` 和标准 `chat/completions`，输入为文本与 Base64 WAV，输出 `message.content` 和 `message.audio.data`。官方示例将输出音频作为 44.1kHz、单声道、16-bit PCM；`audio_bytes` 返回原始 PCM，`audio_wav` 生成标准 RIFF/WAV。模型当前文档给出的上下文为 8K、最大输出 4K，价格和账号权限以官方当前页面为准。

真实调用示例会读取 WAV、调用服务并保存输出：

```bash
cargo run --example glm_4_voice
```

URL、Base64、文件格式、大小、时长和分辨率限制由具体模型定义。SDK 保持传输层中立，不会读取本地路径并隐式上传文件。

## Function Calling

```rust
use rustglm::{ChatCompletionRequest, ChatMessage, FunctionDefinition, Tool};
use serde_json::json;

let weather = Tool::function(
    FunctionDefinition::new(
        "get_weather",
        json!({
            "type": "object",
            "properties": {
                "city": {"type": "string"}
            },
            "required": ["city"]
        }),
    )
    .description("查询指定城市的天气"),
);

let request = ChatCompletionRequest::new("glm-5.2")
    .message(ChatMessage::user("北京天气怎么样"))
    .tools(vec![weather]);
```

执行模型返回的工具后，将原始 assistant 工具调用消息和工具结果一起放回下一轮请求：

```rust
use rustglm::{ChatMessage, ToolCall};

fn tool_messages(calls: Vec<ToolCall>) -> Vec<ChatMessage> {
    let mut messages = vec![ChatMessage::assistant_tool_calls(calls.clone())];
    for call in calls {
        messages.push(ChatMessage::tool_result(
            call.id,
            r#"{"temperature":28,"condition":"sunny"}"#,
        ));
    }
    messages
}
```

函数参数由模型以 JSON 字符串返回，可直接反序列化：

```rust
use serde::Deserialize;
use rustglm::FunctionCall;

#[derive(Deserialize)]
struct WeatherArguments {
    city: String,
}

fn parse(call: &FunctionCall) -> serde_json::Result<WeatherArguments> {
    call.arguments()
}
```

智谱专属工具可使用 `Tool::configured`，不会限制官方未来增加的配置字段：

```rust
use rustglm::Tool;
use serde_json::json;

let search = Tool::configured(
    "web_search",
    "web_search",
    json!({"enable": true, "search_engine": "search_std"}),
);
```

## 异步聊天和任务查询

```rust
use rustglm::{ChatCompletionRequest, ChatMessage, ZhipuClient};

async fn run(client: &ZhipuClient) -> rustglm::Result<()> {
    let request = ChatCompletionRequest::new("glm-5.2")
        .message(ChatMessage::user("生成一份长篇分析"));
    let task = client.async_chat(&request).await?;
    let result = client.async_result(&task.id).await?;
    println!("{}", result.task_status);
    Ok(())
}
```

SDK 不在后台无限轮询。应用应根据自身超时、取消、任务持久化和并发策略查询任务状态。

## Embedding

```rust
use rustglm::{EmbeddingInput, EmbeddingRequest, ZhipuClient};
use serde_json::Map;

async fn embed(client: &ZhipuClient) -> rustglm::Result<Vec<f32>> {
    let request = EmbeddingRequest {
        model: "embedding-3".into(),
        input: EmbeddingInput::Text("Rust 所有权系统".into()),
        dimensions: None,
        encoding_format: None,
        user_id: None,
        request_id: None,
        extra: Map::new(),
    };
    let response = client.embedding(&request).await?;
    Ok(response.data.into_iter().next().map(|item| item.embedding).unwrap_or_default())
}
```

## Rerank

```rust
use rustglm::{RerankRequest, ZhipuClient};

async fn rerank(client: &ZhipuClient) -> rustglm::Result<()> {
    let request = RerankRequest {
        model: "rerank".into(),
        query: "Rust 异步运行时".into(),
        documents: vec!["Tokio".into(), "Serde".into(), "Rayon".into()],
        top_n: Some(2),
        return_documents: Some(true),
        return_raw_scores: None,
        request_id: None,
        user_id: None,
    };
    let response = client.rerank(&request).await?;
    println!("{:?}", response.results);
    Ok(())
}
```

## Tokenizer

```rust
use rustglm::{ChatMessage, TokenizerRequest, ZhipuClient};

async fn tokens(client: &ZhipuClient) -> rustglm::Result<u64> {
    let request = TokenizerRequest {
        model: "glm-4.6".into(),
        messages: vec![ChatMessage::user("计算输入 token")],
        tools: None,
        request_id: None,
        user_id: None,
    };
    Ok(client.tokenizer(&request).await?.usage.total_tokens)
}
```

## 图像生成

```rust
use rustglm::{ImageGenerationRequest, ZhipuClient};
use serde_json::Map;

async fn image(client: &ZhipuClient) -> rustglm::Result<()> {
    let request = ImageGenerationRequest {
        model: "glm-image".into(),
        prompt: "极简风格的 Rust 编程语言海报".into(),
        size: Some("1024x1024".into()),
        quality: None,
        watermark_enabled: Some(true),
        request_id: None,
        user_id: None,
        extra: Map::new(),
    };
    let response = client.create_image(&request).await?;
    println!("{:?}", response.data.first().and_then(|item| item.url.as_deref()));
    Ok(())
}
```

异步图像生成使用 `create_image_async`，返回的任务通过 `async_result` 查询。

## 视频生成

```rust
use rustglm::{VideoGenerationRequest, ZhipuClient};
use serde_json::Map;

async fn video(client: &ZhipuClient) -> rustglm::Result<String> {
    let request = VideoGenerationRequest {
        model: "cogvideox-3".into(),
        prompt: Some("镜头缓慢穿过雨后的未来城市".into()),
        image_url: None,
        quality: Some("quality".into()),
        size: None,
        duration: None,
        fps: None,
        with_audio: Some(true),
        watermark_enabled: Some(true),
        request_id: None,
        user_id: None,
        extra: Map::new(),
    };
    Ok(client.create_video(&request).await?.id)
}
```

视频接口返回异步任务，使用 `async_result` 查询结果。

## 语音转文字

```rust
use rustglm::{TranscriptionRequest, ZhipuClient};

async fn transcribe(client: &ZhipuClient, wav: Vec<u8>) -> rustglm::Result<String> {
    let request = TranscriptionRequest {
        model: "glm-asr-2512".into(),
        file_name: "input.wav".into(),
        file: wav,
        mime_type: Some("audio/wav".into()),
        prompt: None,
        hotwords: vec!["RustGLM".into()],
        request_id: None,
        user_id: None,
    };
    Ok(client.transcribe(request).await?.text)
}
```

## 文字转语音

```rust
use rustglm::{SpeechRequest, ZhipuClient};
use serde_json::Map;

async fn speech(client: &ZhipuClient) -> rustglm::Result<Vec<u8>> {
    let request = SpeechRequest {
        model: "glm-tts".into(),
        input: "欢迎使用 RustGLM".into(),
        voice: "tongtong".into(),
        speed: Some(1.0),
        volume: Some(1.0),
        response_format: Some("wav".into()),
        watermark_enabled: Some(true),
        extra: Map::new(),
    };
    client.speech(&request).await
}
```

## 文件

```rust
use rustglm::{FileUploadRequest, ZhipuClient};

async fn upload(client: &ZhipuClient, bytes: Vec<u8>) -> rustglm::Result<String> {
    let file = client
        .upload_file(FileUploadRequest {
            file_name: "batch.jsonl".into(),
            file: bytes,
            mime_type: Some("application/jsonl".into()),
            purpose: "batch".into(),
        })
        .await?;
    Ok(file.id)
}
```

文件相关方法：

| 方法 | 用途 |
| --- | --- |
| `upload_file` | 上传文件 |
| `files` | 文件列表 |
| `file_content` | 下载文件内容 |
| `delete_file` | 删除文件 |
| `create_file_parse_task` | 创建解析任务 |
| `file_parse_result` | 获取解析结果 |
| `parse_file_sync` | 同步解析 |
| `ocr` | OCR |
| `parse_layout` | 版面解析 |

## Batch

```rust
use rustglm::{BatchCreateRequest, ZhipuClient};

async fn batch(client: &ZhipuClient, file_id: String) -> rustglm::Result<String> {
    let request = BatchCreateRequest {
        input_file_id: file_id,
        endpoint: "/v4/chat/completions".into(),
        completion_window: "24h".into(),
        metadata: None,
    };
    Ok(client.create_batch(&request).await?.id)
}
```

Batch 相关方法包括 `create_batch`、`batches`、`batch` 和 `cancel_batch`。

## 其他官方能力

下列能力提供明确的方法名，并以 `serde_json::Value` 保持与快速变化的官方 Schema 完全兼容：

| 方法 | 官方端点 |
| --- | --- |
| `clone_voice` | `POST voice/clone` |
| `voices` | `GET voice/list` |
| `delete_voice` | `POST voice/delete` |
| `web_search` | `POST web_search` |
| `read_web_page` | `POST reader` |
| `moderate` | `POST moderations` |
| `assistant` | `POST assistant` |
| `assistants` | `POST assistant/list` |
| `assistant_conversations` | `POST assistant/conversation/list` |

这些端点的业务字段仍由智谱官方协议定义，SDK 会提供一致的认证、错误处理、超时和响应解析。

## 泛型官方端点

当官方增加新端点或字段时，不需要等待 SDK 发版：

```rust
use reqwest::Method;
use rustglm::ZhipuClient;
use serde_json::{json, Value};

async fn call(client: &ZhipuClient) -> rustglm::Result<Value> {
    let body = json!({"key": "value"});
    client
        .request_json(Method::POST, "new_endpoint", Some(&body))
        .await
}
```

泛型接口只接受相对于已配置 Base URL 的安全路径，拒绝绝对 URL 和 `..` 路径段，避免认证头被发送到非预期主机。

## OpenAI-compatible 服务

OpenAI、DeepSeek、Qwen、Kimi、Grok 以及其他实现 Chat Completions 兼容协议的服务，可以通过同一客户端接入：

```rust
use rustglm::{
    ChatCompletionRequest, ChatMessage, OpenAiCompatibleConfig,
};

async fn run() -> rustglm::Result<()> {
    let client = OpenAiCompatibleConfig::new(
        "my-provider",
        std::env::var("PROVIDER_API_KEY").unwrap_or_default(),
        "https://provider.example.com/v1",
    )
    .build()?;

    let request = ChatCompletionRequest::new("provider-model")
        .message(ChatMessage::user("你好"));
    let response = client.chat_completion(&request).await?;
    println!("{}", response.text().unwrap_or_default());
    Ok(())
}
```

只设置目标服务明确支持的字段。智谱专属字段不会被 SDK 自动映射成其他厂商的私有协议。

Anthropic Claude 和原生 Gemini API 不是 Chat Completions 协议，不能通过修改 Base URL 正确兼容。它们应实现独立协议适配器，并接入统一 `ChatProvider`。

## ChatProvider

```rust
use rustglm::{ChatCompletionRequest, ChatProvider};

async fn complete(
    provider: &dyn ChatProvider,
    request: ChatCompletionRequest,
) -> rustglm::Result<String> {
    let response = provider.complete(request).await?;
    Ok(response.text().unwrap_or_default().to_owned())
}
```

`ChatProvider` 提供：

- `name`
- `capabilities`
- `complete`
- `stream`

新增 Anthropic、Gemini 或其他非兼容协议时，需要完成厂商请求到统一消息模型的映射、统一响应映射、流事件映射和错误映射，然后实现该 trait。业务层不需要依赖具体厂商客户端。

## 扩展字段

主要请求类型包含公开的 `extra` 字段：

```rust
use rustglm::{ChatCompletionRequest, ChatMessage};

fn request() -> serde_json::Result<ChatCompletionRequest> {
    ChatCompletionRequest::new("glm-5.2")
        .message(ChatMessage::user("你好"))
        .extra("future_parameter", true)
}
```

`extra` 使用 `serde(flatten)` 合并进请求根对象，适合官方新增字段和模型私有参数。稳定且通用的字段会继续升级为强类型成员。

## 错误处理

```rust
use rustglm::{SdkError, ZhipuClient};

async fn handle(client: &ZhipuClient) {
    let result = client.voices().await;
    match result {
        Ok(value) => println!("{value}"),
        Err(SdkError::Api(error)) => {
            eprintln!("status={}", error.status);
            eprintln!("code={:?}", error.code);
            eprintln!("request_id={:?}", error.request_id);
            eprintln!("message={}", error.message);
        }
        Err(error) => eprintln!("{error}"),
    }
}
```

错误类型：

| 类型 | 含义 |
| --- | --- |
| `Configuration` | Base URL、API Key、Header 或客户端配置错误 |
| `Validation` | 请求在发送前被判定为明显非法 |
| `Transport` | DNS、连接、TLS、超时或响应读取错误 |
| `Api` | 服务端返回非 2xx，包含状态码、错误码、消息、请求 ID 和原始正文 |
| `Decode` | 成功响应无法按目标类型解析，包含原始正文 |
| `Stream` | SSE 事件无法解析 |
| `Unsupported` | Provider 不支持目标能力 |
| `Agent` | 智能体无响应、超过步数上限或运行状态错误 |
| `Tool` | 工具未注册、参数非法或执行失败 |

API Key 不实现公开 Debug 输出。错误消息不会主动包含认证头。

## 并发、取消和性能

- `ZhipuClient` 和 `OpenAiCompatibleClient` 都实现 `Clone`。
- 克隆客户端只增加共享引用，不复制连接池。
- 每个请求都是独立 Future，可直接通过 Tokio 并发执行。
- 丢弃请求 Future 会取消等待过程。
- 丢弃流会停止继续消费响应。
- HTTP 与 Realtime 客户端不维护全局聊天状态；只有调用者显式创建的 `Conversation`、JWT 缓存和内存向量库持有实例级共享状态。
- `AgentRuntime` 的状态属于实例；不同用户或租户必须使用独立实例或由应用执行严格隔离。
- JSON 仅在请求发送前序列化一次，重试复用请求字节。
- SSE 使用增量字节缓冲，不要求网络块与 UTF-8 或事件边界对齐。
- Realtime 使用有界 Tokio channel 隔离 WebSocket I/O 与媒体生产者/消费者，容量可配置。
- 大文件上传当前接收 `Vec<u8>`，调用者应根据文件限制控制内存占用。

## 官方 v4 端点覆盖

当前官方 OpenAPI 中的 v4 路由可通过强类型方法、明确 `Value` 方法或 `request_json` 使用：

```text
POST   chat/completions
POST   async/chat/completions
POST   videos/generations
POST   async/images/generations
GET    async-result/{id}
POST   images/generations
POST   audio/transcriptions
POST   audio/speech
POST   voice/clone
GET    voice/list
POST   voice/delete
POST   embeddings
POST   rerank
POST   tokenizer
POST   layout_parsing
POST   web_search
POST   reader
POST   moderations
POST   files
GET    files
DELETE files/{file_id}
GET    files/{file_id}/content
POST   files/parser/create
GET    files/parser/result/{taskId}/{format_type}
POST   files/parser/sync
POST   files/ocr
POST   batches
GET    batches
GET    batches/{batch_id}
POST   batches/{batch_id}/cancel
POST   assistant
POST   assistant/list
POST   assistant/conversation/list
```

Agent API 还覆盖：

```text
POST   /api/v1/agents
POST   /api/v1/agents/async-result
POST   /api/v1/agents/conversation
POST   /api/zrag/agent/chat
```

## 与 0.1.x 的差异

`0.2.0` 是重新设计后的 SDK 接口：

- crate 导入名改为 `rustglm`
- 使用 `ZhipuClient` 代替交互式 `RustGLM`
- 支持官方 Python、Java 和 Realtime SDK 规则的本地 JWT，并保留显式 Bearer 模式
- 不再访问 NTP 服务
- 不再使用 `Constants.toml`
- 不再通过 `SSE#问题`、`glm-4v:文本@图片` 等字符串协议表达请求
- 不再自动写入 `chatglm_history.json`
- 方法返回结构化 `Result<T, SdkError>`
- 聊天历史由 `Vec<ChatMessage>` 明确表达
- 多模态由 `Vec<ContentPart>` 明确表达
- 流式接口返回真正的异步 Stream
- 上下文记忆改为显式可选的 `Conversation` 和可插拔语义记忆接口
- 新增 GLM-Realtime WebSocket 音视频协议客户端

0.1.x 的交互式调用、NTP、自定义字符串协议和历史记录源码已经删除，不再作为未编译死代码保留。

## 测试与验证

```bash
cargo fmt --all --check
cargo test --all-targets
cargo clippy --all-targets -- -D warnings
cargo test --all-targets --no-default-features
cargo clippy --all-targets --no-default-features -- -D warnings
cargo llvm-cov --summary-only
cargo doc --no-deps
```

当前默认 feature 实测为 66 个离线测试全部通过，真实 API 测试 2 个默认忽略。关闭 Realtime 后为 62 个离线测试全部通过，1 个真实 API 测试默认忽略。`cargo llvm-cov --summary-only` 的实际行覆盖率：

| 模块 | 行覆盖率 |
| --- | ---: |
| `agent.rs` | 96.18% |
| `auth.rs` | 94.35% |
| `client.rs` | 97.76% |
| `memory.rs` | 94.18% |
| `provider.rs` | 100.00% |
| `realtime.rs` | 94.42% |
| `transport.rs` | 95.99% |
| `types.rs` | 95.93% |
| `voice.rs` | 96.36% |
| 总计 | 96.10% |

离线测试使用 `127.0.0.1` 临时 HTTP 与 WebSocket 服务，不读取环境变量中的密钥、不访问智谱、不产生费用。覆盖内容包括：

- JWT Header、Payload、HS256 签名、有效期、刷新缓存和 Bearer 模式
- 多模态、Function Calling、全部公开 HTTP 端点族和参数校验
- SSE 分块、UTF-8 边界、`[DONE]`、错误事件、重试和结构化 API 错误
- 无状态/最近消息/语义记忆、int8 量化、余弦检索、upsert、快照和恢复
- Realtime WebSocket Authorization、会话更新、音频、视频、VAD、上下文项和响应事件
- OpenAI-compatible 与统一 `ChatProvider`
- 官方 Agent v1、ReAct Agent SSE、角色清单、工具循环、步数保护和语义记忆
- GLM-4-Voice 请求编码、音频 Base64 解码和 PCM/WAV 封装
- 默认 Realtime 与 `--no-default-features` 轻量构建

真实文本 API 测试必须由调用者明确提供 Key 并指定 ignored 测试：

```powershell
$env:ZHIPU_API_KEY="your-key-id.your-secret"
cargo test --test live_zhipu -- --ignored --nocapture
```

只验证 Realtime JWT、WebSocket 握手和 `session.created`：

```powershell
$env:ZHIPU_API_KEY="your-key-id.your-secret"
cargo test --test live_realtime -- --ignored --nocapture
```

完整真实音视频链路通过 `cargo run --example realtime_audio_video` 手动验证。仓库没有密钥和媒体设备时不会声称这些真实服务测试已经通过。

## 安全建议

- API Key 只从安全的环境变量或密钥管理系统加载。
- 不要在 Debug、日志、Tracing Span 或错误上下文中记录认证头。
- 对最终用户设置稳定且不包含隐私信息的 `user_id`。
- 为业务请求设置唯一 `request_id` 并记录官方返回的请求 ID。
- 工具调用参数必须反序列化并验证，不能直接拼接成 shell 或数据库语句。
- 下载和模型生成的 URL 应按不可信输入处理。
- 使用多模态 URL 时限制允许的协议、域名和文件大小。
- 生产环境应设置应用级并发限制、总超时和费用配额。
- 自动重试生成请求前确认幂等性和计费行为。
- 语义记忆快照包含原始对话文本，必须执行租户隔离、访问控制、加密和数据删除策略。
- 召回内容和 Realtime 转写都属于不可信输入，拼入提示词或执行工具前必须验证。
- 麦克风、摄像头和屏幕采集必须在获得用户明确授权后启动，并提供可见的停止控制。

## 官方资料

- [智谱 AI 开放文档](https://docs.bigmodel.cn/)
- [快速开始](https://docs.bigmodel.cn/cn/guide/start/quick-start)
- [模型概览](https://docs.bigmodel.cn/cn/guide/start/model-overview)
- [对话补全 API](https://docs.bigmodel.cn/api-reference/模型-api/对话补全)
- [智谱官方 OpenAPI](https://docs.bigmodel.cn/openapi/openapi.json)
- [GLM-Realtime 官方文档](https://docs.bigmodel.cn/cn/guide/models/sound-and-video/glm-realtime)
- [GLM-4-Voice 官方文档](https://docs.bigmodel.cn/cn/guide/models/sound-and-video/glm-4-voice)
- [智能体对话 API](https://docs.bigmodel.cn/api-reference/agent-api/智能体对话)
- [问答 Agent 对话 API](https://docs.bigmodel.cn/api-reference/agent-api/问答-agent-对话（流式）)
- [GLM-Realtime 官方 SDK](https://github.com/MetaGLM/glm-realtime-sdk)

模型名称、上下文长度、输出上限、价格和账号可用范围会变化，应以智谱官方文档和当前账号控制台为准。
