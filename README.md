# RustGLM

English | [简体中文](README_zh.md)

[![CI](https://github.com/blueokanna/rustglm/actions/workflows/ci.yml/badge.svg)](https://github.com/blueokanna/rustglm/actions/workflows/ci.yml)
[![Release](https://github.com/blueokanna/rustglm/actions/workflows/release.yml/badge.svg)](https://github.com/blueokanna/rustglm/actions/workflows/release.yml)
[![Crates.io](https://img.shields.io/crates/v/rustglm.svg)](https://crates.io/crates/rustglm)
[![Documentation](https://docs.rs/rustglm/badge.svg)](https://docs.rs/rustglm)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)

RustGLM is an async Rust SDK for the Zhipu AI open platform. It covers the usual chat surface (GLM-5 typed requests, SSE streaming, ToolStream assembly) and then some: batch jobs, knowledge bases, file parsing and OCR, image/video generation, voice, a bidirectional Realtime WebSocket client, a local agent runtime with semantic memory, and a standalone MCP client built on `rmcp`.

I wrote it for backend services, so the crate keeps its hands off your machine. No hidden config file discovery, no implicit disk writes, no telemetry. Credentials, retries, persistence, and client lifetime are yours to control.

## Quick start

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
    .message(ChatMessage::user("Explain ownership in one paragraph."));
let response = client.chat_completion(&request).await?;
println!("{}", response.text().unwrap_or_default());
# Ok(())
# }
```

## Requirements

- Rust 1.88 or later (2024 edition)
- Tokio for anything async
- A Zhipu API key, or a bearer token you already obtained

## What the crate does and doesn't do

A few ground rules so there are no surprises in production:

- No implicit disk I/O. File and RAG uploads take caller-owned bytes; the crate never opens a path itself.
- No implicit credentials. API keys are constructor arguments. `EnvironmentSecretResolver` is an explicit opt-in agent utility, not magic.
- No network traffic you didn't ask for. Constructing config values does zero I/O; requests fire only when you await an endpoint method.
- No telemetry, no metadata endpoints, no NTP. JWT signing only reads the local clock.
- HTTP retries are off by default. If you turn them on, only idempotent methods (GET, HEAD, DELETE, OPTIONS, PUT) are retried on connection/timeout failures, so a POST that already landed server-side isn't replayed blindly.
- Credentials only travel over TLS. A plaintext `http://` base URL is rejected for non-local hosts unless you opt in with `HttpConfig::allow_insecure(true)` — same policy MCP and Realtime already had.

## Feature flags

Default features cover the full Zhipu surface; the MCP client is opt-in because it pulls in `rmcp`.

| Feature | Default | What you get |
| --- | ---: | --- |
| `agents` | yes | Official agents, assistant endpoints, local agent runtime |
| `audio` | yes | GLM-4-Voice, transcription, speech, voice management |
| `batch` | yes | Typed Batch API: create, list, inspect, cancel |
| `files` | yes | Upload/download/delete, parsing, OCR, layout parsing |
| `images` | yes | Image generation |
| `mcp` | no | Standalone Streamable HTTP MCP client (`rmcp`) |
| `rag` | yes | Retrieval agent, knowledge bases, document management |
| `realtime` | yes | Typed bidirectional WebSocket client |
| `tools` | yes | Hosted tool types, web ops, ToolStream assembly |
| `video` | yes | Video generation |
| `full` | no | Everything, including `mcp` |

Minimal chat-only build:

```toml
[dependencies]
rustglm = { version = "1.0", default-features = false }
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

For a bigger install, pick what you need:

```toml
[dependencies]
rustglm = {
    version = "1.0",
    default-features = false,
    features = ["batch", "mcp", "rag", "realtime", "tools"]
}
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

## Authentication

`ZhipuClient::new`, `ZhipuConfig::new`, and `RealtimeConfig::new` all take credentials directly.

- `key_id.secret` is treated as a combined API key and signed as an HS256 JWT (cached, refreshed before expiry).
- Anything else non-empty is treated as an opaque bearer token.
- Use `ZhipuAuthentication::jwt` or `ZhipuAuthentication::bearer` when you need to be explicit.

Don't commit credentials. Read them from env, a secret manager, or a workload identity provider — that part is on your side.

## Typed GLM-5 chat

Marker types and a sealed capability trait mean the typed API refuses unsupported fields at compile time. You can't send `reasoning_effort` to a model that doesn't support it, and a request can't reach the network before it has user or tool input.

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

The supported markers are listed below. For models that aren't in the table yet (new releases, private deployments), use `ChatCompletionRequest` directly with the model ID as a string.

| Text model | Marker | Thinking | Reasoning effort | ToolStream |
| --- | --- | :---: | :---: | :---: |
| `glm-5.3` | `Glm53` | yes | yes | yes |
| `glm-5.2` | `Glm52` | yes | yes | yes |
| `glm-5.1` | `Glm51` | yes | no | yes |
| `glm-5.1-highspeed` | `Glm51Highspeed` | yes | no | yes |
| `glm-5-turbo` | `Glm5Turbo` | yes | no | yes |
| `glm-5` | `Glm5` | yes | no | yes |
| `glm-4.7` | `Glm47` | yes | no | yes |
| `glm-4.7-flash` | `Glm47Flash` | yes | no | no |
| `glm-4.7-flashx` | `Glm47FlashX` | yes | no | no |
| `glm-4.6` | `Glm46` | yes | no | yes |
| `glm-4.5-air` | `Glm45Air` | yes | no | no |
| `glm-4.5-airx` | `Glm45AirX` | yes | no | no |
| `glm-4.5-flash` | `Glm45Flash` | yes | no | no |
| `glm-4-flash-250414` | `Glm4Flash250414` | no | no | no |
| `glm-4-flashx-250414` | `Glm4FlashX250414` | no | no | no |
| `glm-4-long` | `Glm4Long` | no | no | no |
| `charglm-4` | `Charglm4` | no | no | no |
| `emohaa` | `Emohaa` | no | no | no |

| Vision model | Marker | Thinking | ToolStream |
| --- | --- | :---: | :---: |
| `glm-5v-turbo` | `Glm5vTurbo` | yes | no |
| `autoglm-phone` | `AutoGlmPhone` | no | no |
| `glm-4.6v` | `Glm46v` | yes | no |
| `glm-4.6v-flash` | `Glm46vFlash` | yes | no |
| `glm-4.6v-flashx` | `Glm46vFlashX` | yes | no |
| `glm-4v-flash` | `Glm4vFlash` | no | no |
| `glm-4.1v-thinking-flash` | `Glm41vThinkingFlash` | yes | no |
| `glm-4.1v-thinking-flashx` | `Glm41vThinkingFlashX` | yes | no |
| `glm-ocr` | `GlmOcr` | no | no |
| `glm-4.1v-thinking` | `Glm41vThinking` | yes | no |

`ReasoningEffort`, `Thinking`, ToolStream, tools, and vision input only exist on markers that declare the capability, so an unsupported field never reaches the transport through the typed path.

## ToolStream

ToolStream turns fragmented SSE function-call deltas into complete typed calls while keeping text, reasoning, usage, and stream errors in the same stream. It also enforces sane limits: arguments per call and concurrent pending calls are bounded, and a call can't be emitted as "complete" unless its arguments parse as valid JSON.

```rust,no_run
use futures_util::StreamExt;
use rustglm::{Glm53, ToolStreamEvent, TypedChatRequest, ZhipuClient};

# async fn run() -> rustglm::Result<()> {
let client = ZhipuClient::new("token")?;
let request = TypedChatRequest::<Glm53>::new()
    .tool_stream()
    .user("Check the deployment status.");
let mut stream = client.typed_chat_tool_stream(&request).await?;

while let Some(event) = stream.next().await {
    if let ToolStreamEvent::ToolCallCompleted(call) = event? {
        println!("{} {}", call.name, call.arguments);
    }
}
# Ok(())
# }
```

## Local agent runtime

Beyond the official hosted agents, there's a self-hosted `AgentRuntime` that runs a manifest-driven loop over any `ChatProvider`: persona prompt, history policy, registered tools, and optional semantic memory. The runtime is defensible by default:

- `max_steps` is capped; total tool executions and per-tool output bytes are budgeted.
- Optional `run_timeout` and `tool_timeout` keep a stuck model or a hung tool from blocking a request forever.
- Recalled memory is injected as untrusted context, explicitly framed as data — never as instructions. This limits the blast radius of prompt-injection content that ends up in a memory store.
- Errors are structured (`StepLimit`, `BudgetExceeded`, `NoOutput`, `ToolError::NotRegistered`, ...), so you can branch on them instead of string-matching.

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

## Content moderation

The `moderate_content` method sends typed text, image, audio, or video content to the moderation model and returns structured risk results. Text input is capped at 2000 characters; media URLs must be absolute HTTP(S) URLs, checked before anything leaves your process.

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

The `batch` feature provides typed completion windows and statuses. Batch input files are uploaded explicitly through the `files` API.

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

Available methods are `create_batch`, `batches`, `batch`, and `cancel_batch`. List limits outside `1..=100` are rejected with `BatchError::InvalidLimit` before any network I/O.

## Knowledge bases and RAG

The `rag` feature follows the official knowledge-base OpenAPI paths: knowledge-base CRUD, capacity, retrieval, document list/detail, in-memory file upload, URL ingestion, deletion, document images, and re-embedding. Upload and callback URLs are validated before they go anywhere — they must be absolute HTTP(S) URLs without embedded credentials.

```rust,no_run
use rustglm::{
    DocumentChunking, KnowledgeCreateRequest, KnowledgeEmbeddingModel,
    KnowledgeRetrieveRequest, RagDocumentUpload, UrlDocument,
    UrlDocumentUploadRequest, ZhipuClient,
};

# async fn run() -> rustglm::Result<()> {
let client = ZhipuClient::new("token")?;
let created = client
    .create_knowledge_base(&KnowledgeCreateRequest::new(
        "engineering-runbooks",
        KnowledgeEmbeddingModel::Embedding3Pro,
    ))
    .await?;
let knowledge_id = created.data.expect("successful response").id;

client
    .upload_knowledge_document(
        &knowledge_id,
        RagDocumentUpload::from_bytes("runbook.md", b"rollback procedure".to_vec()),
    )
    .await?;

let results = client
    .retrieve_knowledge(&KnowledgeRetrieveRequest::new(
        "How do we roll back?",
        [knowledge_id],
    ))
    .await?;
println!("{} matches", results.data.unwrap_or_default().len());
# Ok(())
# }
```

`RagDocumentUpload::from_bytes` has no path-based constructor on purpose. File reads, size limits, encryption, tenant boundaries, and retention stay in your hands.

## MCP client

The `mcp` feature is a standalone Model Context Protocol client (separate from `McpTool`, which configures a hosted MCP tool inside a model request). Protocol framing, initialization, tools, resources, prompts, and the Streamable HTTP transport come from the official `rmcp` crate.

```rust,no_run
use rustglm::McpClientConfig;

# async fn run() -> rustglm::Result<()> {
let mut client = McpClientConfig::new("https://mcp.example.com/mcp")
    .bearer_token("tenant-token")
    .header("x-tenant-id", "acme")?
    .connect()
    .await?;

for tool in client.list_tools().await? {
    println!("{}", tool.name);
}

client.close().await?;
# Ok(())
# }
```

Security defaults worth knowing:

- Only absolute HTTP(S) endpoints. Plain `http://` is rejected for non-local hosts unless you opt in with `allow_insecure(true)`.
- URLs with embedded credentials are rejected outright.
- `Debug` output redacts the bearer token and only prints header names, never values.
- Redirects are disabled and the SDK-created client has connect/request timeouts.
- SSE retry and automatic expired-session reinitialization are off by default.
- You can inject your own `reqwest::Client` for proxy, TLS, DNS, or policy control.

## Realtime WebSocket

The `realtime` feature is a typed bidirectional WebSocket client. Audio and video move as caller-owned byte slices and are encoded in memory.

```rust,no_run
use rustglm::{
    RealtimeConfig, RealtimeRequest, TypedRealtimeSession,
};

# async fn run() -> rustglm::Result<()> {
let mut connection = RealtimeConfig::new("token").connect().await?;
let session = TypedRealtimeSession::default()
    .instructions("Be concise.")
    .server_vad();

connection
    .send_request(&RealtimeRequest::session_update(session)?)
    .await?;
connection
    .send_request(&RealtimeRequest::append_audio(&[0_u8; 320])?)
    .await?;

while let Some(event) = connection.next_typed_event().await {
    let event = event?;
    if let Some(text) = event.delta_text() {
        print!("{text}");
    }
}
# Ok(())
# }
```

Transport hardening on this path:

- Default endpoint is `wss://`. Plain `ws://` is refused unless you explicitly set `allow_insecure(true)`.
- Message and frame size limits are enforced at the WebSocket layer.
- The event loop never blocks on a full outgoing channel — a slow consumer drops events instead of deadlocking the connection, pings, or close.
- Writes are time-bounded and `close()` waits at most a few seconds for the background task.

The client also supports typed session tools, function-call output, response options, transcription sessions, client/server VAD, cancellation, audio commit/clear, video frames, and explicit close.

## Errors

`SdkError` is the single public error type. Domain errors stay as typed enums, so you can match on them instead of parsing strings.

```rust
use rustglm::{BatchError, SdkError};

fn classify(error: SdkError) {
    match error {
        SdkError::Batch(BatchError::InvalidLimit(limit)) => {
            eprintln!("invalid batch limit: {limit}");
        }
        SdkError::Api(api) => {
            eprintln!("HTTP {} request_id={:?}", api.status, api.request_id);
        }
        SdkError::PayloadTooLarge { limit, .. } => {
            eprintln!("response exceeded {limit} bytes");
        }
        other => eprintln!("{other}"),
    }
}
```

The envelope separates configuration, validation, transport, timeout, API, decode, stream, WebSocket, unsupported capability, agent, tool, Batch, RAG, and MCP failures. `ApiError` keeps the HTTP status, provider code, message, and request ID. It also keeps the raw response body — useful for debugging, but that body can contain model output or other sensitive data, so think twice before logging it verbatim.

## HTTP policy

`HttpConfig` controls request timeout, connect timeout, pool idle timeout, response size limit, user agent, default headers, retry policy, and an optional caller-built `reqwest::Client`.

- Retries are off by default; if enabled, only the status codes you configure are retried, and connection/timeout retries apply to idempotent methods only.
- Response bodies are read with a size cap (`max_response_bytes`, 64 MiB default) so a misbehaving or malicious endpoint can't balloon your process memory. Error responses are read under a separate 64 KiB cap. The same idea applies to SSE events (16 MiB per event, at most 4096 data lines per event) and streamed tool arguments (1 MiB per call).
- Base URLs must be HTTPS for non-local hosts. Plain `http://` works for loopback and private ranges (local testing, LAN proxies) and elsewhere only after `allow_insecure(true)`.
- JWT token caching uses a monotonic clock, so a backwards system-time jump can't extend the life of an expired token.
- Error bodies are scrubbed before they land in `ApiError`: Zhipu `id.secret` keys, bearer tokens, and the configured credential itself are replaced with `[FILTERED]` so secrets don't leak through your logs.

## API coverage

A quick index of the public surface. Methods that accept `nextjson::Value` exist on purpose — provider schemas move faster than this crate can ship typed builders.

| Area | Feature | Public operations |
| --- | --- | --- |
| Chat and streams | core; `tools` for ToolStream | `chat_completion`, `chat_completion_stream`, `chat_tool_stream`, `typed_chat_completion`, `typed_chat_completion_stream`, `typed_chat_tool_stream` |
| Async and vector APIs | core | `async_chat`, `async_result`, `embedding`, `rerank`, `tokenizer` |
| Images and video | `images`, `video` | `create_image`, `create_image_async`, `create_video` |
| Audio and voice | `audio` | `glm_4_voice`, `transcribe`, `speech`, `clone_voice`, `voices`, `delete_voice` |
| Hosted tools | `tools` | `web_search`, `read_web_page`, `moderate`, `moderate_content` |
| Files and document processing | `files` | `upload_file`, `files`, `file_content`, `delete_file`, `create_file_parse_task`, `file_parse_result`, `parse_file_sync`, `ocr`, `parse_layout` |
| Batch | `batch` | `create_batch`, `batches`, `batch`, `cancel_batch` |
| Official agents and assistants | `agents` | `official_agent`, `official_agent_stream`, `official_agent_async_result`, `official_agent_conversation`, `assistant`, `assistants`, `assistant_conversations` |
| Knowledge bases and retrieval | `rag` | `create_knowledge_base`, `knowledge_bases`, `knowledge_base`, `update_knowledge_base`, `delete_knowledge_base`, `knowledge_capacity`, `retrieve_knowledge`, `knowledge_documents`, `upload_knowledge_document`, `upload_knowledge_urls`, `knowledge_document`, `delete_knowledge_document`, `knowledge_document_images`, `reembed_knowledge_document`, `retrieval_agent_stream` |
| Protocol escape hatch | core | `request_json` on both `ZhipuClient` and `OpenAiCompatibleClient` |
| Standalone MCP | `mcp` | `McpClientConfig::connect`, plus typed tool, resource, prompt, and Streamable HTTP operations from `rmcp` |
| Realtime | `realtime` | `RealtimeConfig::connect`, typed requests/events, VAD, media buffers, function-call output, cancellation, and explicit close |

`request_json` exists for provider fields and endpoints that aren't typed yet. It only takes relative paths on the configured base URL — absolute URLs and `..` segments are rejected.

## Examples

There are 36 runnable examples in `examples/`, one per endpoint family with a few lifecycle ones that string related operations together. `cargo check --all-targets --all-features` compiles all of them without touching the network.

### Chat, models, and vectors

| Example | What it shows |
| --- | --- |
| [`chat_completion`](examples/chat_completion.rs) | `chat_completion` |
| [`chat_stream`](examples/chat_stream.rs) | `chat_completion_stream` |
| [`typed_chat`](examples/typed_chat.rs) | `typed_chat_completion`, thinking, reasoning effort |
| [`multimodal_chat`](examples/multimodal_chat.rs) | vision content parts and image URL input |
| [`function_calling`](examples/function_calling.rs) | function schemas and `Tool::function` |
| [`tool_stream`](examples/tool_stream.rs) | `typed_chat_tool_stream` and assembled function-call deltas |
| [`async_chat`](examples/async_chat.rs) | `async_chat`, `async_result` |
| [`embedding`](examples/embedding.rs) | `EmbeddingRequest`, `embedding` |
| [`rerank`](examples/rerank.rs) | `RerankRequest`, `rerank` |
| [`tokenizer`](examples/tokenizer.rs) | `TokenizerRequest`, `tokenizer` |
| [`openai_compatible`](examples/openai_compatible.rs) | `OpenAiCompatibleConfig`, `ChatProvider` |

### Media, files, and document processing

| Example | What it shows |
| --- | --- |
| [`image_generation`](examples/image_generation.rs) | `create_image`, `create_image_async` |
| [`video_generation`](examples/video_generation.rs) | `create_video`, async task ID |
| [`speech`](examples/speech.rs) | `SpeechRequest`, `speech` |
| [`transcription`](examples/transcription.rs) | `TranscriptionRequest`, `transcribe` |
| [`glm_4_voice`](examples/glm_4_voice.rs) | GLM-4-Voice input and WAV output |
| [`voice_management`](examples/voice_management.rs) | `clone_voice`, `voices`, `delete_voice` |
| [`file_management`](examples/file_management.rs) | `upload_file`, `files`, `file_content`, `delete_file` |
| [`file_parsing`](examples/file_parsing.rs) | `create_file_parse_task`, `file_parse_result`, `parse_file_sync` |
| [`document_understanding`](examples/document_understanding.rs) | `ocr`, `parse_layout` |

### Batch, hosted tools, and RAG

| Example | What it shows |
| --- | --- |
| [`web_search`](examples/web_search.rs) | `web_search` |
| [`hosted_tools`](examples/hosted_tools.rs) | `read_web_page`, `moderate` |
| [`file_batch`](examples/file_batch.rs) | upload JSONL and `create_batch` |
| [`batch_management`](examples/batch_management.rs) | create, list, retrieve, and cancel Batch operations |
| [`knowledge_base`](examples/knowledge_base.rs) | `create_knowledge_base` |
| [`knowledge_management`](examples/knowledge_management.rs) | knowledge-base list, detail, update, capacity, and delete |
| [`knowledge_documents`](examples/knowledge_documents.rs) | document list, upload, URL ingestion, detail, images, re-embed, and delete |
| [`knowledge_retrieval`](examples/knowledge_retrieval.rs) | `retrieve_knowledge` |
| [`retrieval_agent`](examples/retrieval_agent.rs) | `retrieval_agent_stream` |

### Agents, MCP, and Realtime

| Example | What it shows |
| --- | --- |
| [`official_agent`](examples/official_agent.rs) | typed official Agent v1 invocation |
| [`official_agent_lifecycle`](examples/official_agent_lifecycle.rs) | Agent stream, async result, and conversation operations |
| [`assistants`](examples/assistants.rs) | Assistant invoke, list, and conversations |
| [`custom_agent`](examples/custom_agent.rs) | local agent runtime with an application tool |
| [`interactive_chat`](examples/interactive_chat.rs) | multi-turn runtime and optional semantic memory |
| [`mcp_client`](examples/mcp_client.rs) | MCP tools, resources, prompts, and close |
| [`realtime_audio_video`](examples/realtime_audio_video.rs) | Realtime PCM/WAV, optional JPEG frames, typed events |

Run one with `cargo run --example <name> -- <args>`. The MCP example needs the feature: `cargo run --example mcp_client --features mcp -- <endpoint>`. Most Zhipu examples want `ZHIPU_API_KEY`; `openai_compatible` uses `OPENAI_COMPATIBLE_BASE_URL` and `OPENAI_COMPATIBLE_API_KEY`. Running examples spends quota and can create or delete remote resources — don't point them at a production account casually.

## CI and releases

CI runs formatting, Clippy with warnings denied, builds across no-default/default/all/individual enterprise feature sets, tests and doctests, docs with warnings denied, package construction from the committed lockfile, and enforces 90% line coverage on the all-feature build with LCOV plus text-summary artifacts.

The release workflow runs on `v*` tags. You can also trigger it manually from the Actions UI by selecting a commit or branch and entering `v<Cargo.toml version>` as the `tag` input — the workflow checks out that exact revision rather than assuming the tag exists. It rejects version mismatches, runs every release gate, builds the `.crate`, writes `SHA256SUMS`, creates the annotated tag if missing (an existing tag is only accepted if it points at the verified commit), then optionally publishes to crates.io when `CARGO_REGISTRY_TOKEN` is configured and creates or updates the GitHub Release.

```bash
# Bump Cargo.toml first, then:
git tag -s v1.0.0 -m "RustGLM v1.0.0"
git push origin v1.0.0
```

No API key or registry token lives in the repo. `CARGO_REGISTRY_TOKEN` should only ever exist as a GitHub Actions secret.

## Testing and coverage

```bash
cargo fmt --all -- --check
cargo test --all-targets --no-default-features
cargo test --all-targets
cargo test --all-targets --all-features
cargo clippy --all-targets --all-features -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --all-features --no-deps
cargo package --locked
```

Coverage is enforced at 90% line coverage in CI, with LCOV and text-summary artifacts published per run. Reproduce it locally:

```bash
cargo coverage       # summary plus the 90% line threshold
cargo coverage-lcov  # target/rustglm-lcov.info plus the same threshold
```

Full per-module numbers live in [docs/COVERAGE.md](docs/COVERAGE.md) — treat the CI artifacts as authoritative for a given commit rather than a stale snapshot. The 2 live tests (`live_zhipu`, `live_realtime`) are ignored by default; run them deliberately with a real key:

```powershell
$env:ZHIPU_API_KEY = "key_id.secret"
cargo test --test live_zhipu -- --ignored --nocapture
cargo test --test live_realtime -- --ignored --nocapture
```

## License

Apache License 2.0. See [LICENSE](LICENSE).
