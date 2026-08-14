# RustGLM

English | [简体中文](README_zh.md)

[![CI](https://github.com/blueokanna/rustglm/actions/workflows/ci.yml/badge.svg)](https://github.com/blueokanna/rustglm/actions/workflows/ci.yml)
[![Release](https://github.com/blueokanna/rustglm/actions/workflows/release.yml/badge.svg)](https://github.com/blueokanna/rustglm/actions/workflows/release.yml)
[![Crates.io](https://img.shields.io/crates/v/rustglm.svg)](https://crates.io/crates/rustglm)
[![Documentation](https://docs.rs/rustglm/badge.svg)](https://docs.rs/rustglm)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)

RustGLM is an unofficial, async Rust SDK for the Zhipu AI Open Platform. It provides typed GLM-5 requests, SSE and ToolStream assembly, bidirectional Realtime WebSocket sessions, Batch API operations, knowledge-base management, and an MCP client built on the official Rust MCP SDK.

The crate targets production backends. Network policy, persistence, credentials, retries, and client lifetime remain visible to the application.

## Quick start

Add the default SDK and Tokio runtime:

```toml
[dependencies]
rustglm = "1.0.0"
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

Set the credential and run a completion:

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

- Rust 1.88 or later
- Rust 2024 Edition
- Tokio for async execution
- A Zhipu API key or an already-issued bearer token for Zhipu endpoints

The Cargo package and Rust crate name are both `rustglm`.

## Side-effect contract

RustGLM does not perform implicit disk I/O.

- The library does not create directories, discover configuration files, write logs, cache responses, persist conversations, or save audio and video.
- File and RAG upload APIs accept caller-owned bytes. They never open a path internally.
- Responses, SSE frames, Realtime media, memory snapshots, and tool events remain in memory or async streams.
- The library does not read API keys from environment variables. Credentials are constructor arguments. `EnvironmentSecretResolver` is an explicit, opt-in agent utility.
- The library never contacts NTP, metadata, telemetry, or model-discovery services. JWT signing uses the local system clock only.
- HTTP retries default to zero. MCP SSE retry and expired-session reinitialization default to disabled.
- Constructing configuration values performs no network I/O. Zhipu requests run only when an endpoint method is awaited; an MCP or Realtime connection starts only when `connect` is awaited.

Examples may explicitly read environment variables or local files. Those are application-level actions and are not performed by the SDK.

## Feature flags

Default features preserve the broad Zhipu API surface while keeping the standalone MCP protocol client opt-in.

| Feature | Default | API surface |
| --- | ---: | --- |
| `agents` | yes | Official agents, assistant endpoints, and local agent runtime |
| `audio` | yes | GLM-4-Voice, transcription, speech, and voice operations |
| `batch` | yes | Typed Batch API create, list, inspect, and cancel operations |
| `files` | yes | File upload/download/delete, parsing, OCR, and layout parsing |
| `images` | yes | Image generation |
| `mcp` | no | Standalone Streamable HTTP MCP client backed by `rmcp` |
| `rag` | yes | Retrieval Agent plus knowledge-base and document management |
| `realtime` | yes | Typed bidirectional WebSocket client |
| `tools` | yes | Hosted tool types, web operations, and ToolStream assembly |
| `video` | yes | Video generation |
| `full` | no | Enables every feature, including `mcp` |

Minimal HTTP chat client:

```toml
[dependencies]
rustglm = { version = "1.0.0", default-features = false }
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

Selected enterprise APIs:

```toml
[dependencies]
rustglm = {
    version = "1.0.0",
    default-features = false,
    features = ["batch", "mcp", "rag", "realtime", "tools"]
}
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

All APIs:

```toml
[dependencies]
rustglm = { version = "1.0.0", features = ["full"] }
```

## Authentication

`ZhipuClient::new`, `ZhipuConfig::new`, and `RealtimeConfig::new` accept credentials directly.

- `key_id.secret` is treated as a Zhipu combined API key and is signed as an HS256 JWT.
- Any other non-empty value is treated as an opaque bearer token.
- Use `ZhipuAuthentication::jwt` or `ZhipuAuthentication::bearer` when automatic selection is not appropriate.

Do not commit credentials. Reading a secret from a process environment, secret manager, or workload identity provider is the application's responsibility.

## Typed GLM-5 chat

Marker types, sealed capability traits, and request typestate prevent unsupported operations from being sent through the typed API. A request cannot be passed to a typed completion method before it contains user or tool input.

```rust,no_run
use rustglm::{
    Glm53, ReasoningEffort, Thinking, TypedChatRequest, ZhipuClient,
};

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

### Supported chat models

The table describes the compile-time typed API. Raw `ChatCompletionRequest` remains available for newly released or private model IDs.

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

`ReasoningEffort`, `Thinking`, ToolStream, tools, and vision input are exposed only on marker types that declare those capabilities. This prevents an unsupported field from reaching the transport through the typed API.

`ChatCompletionRequest` remains available as a forward-compatible raw request when a newly released field has not yet received a typed builder.

## ToolStream

ToolStream combines fragmented SSE function-call deltas into complete typed calls while preserving text, reasoning, usage, and stream errors.

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

Available methods are `create_batch`, `batches`, `batch`, and `cancel_batch`. List limits outside `1..=100` produce `BatchError::InvalidLimit` before network I/O.

## Knowledge bases and RAG

The `rag` feature follows the official knowledge-base OpenAPI paths. It covers knowledge-base CRUD, capacity, retrieval, document listing and detail, in-memory file upload, URL ingestion, deletion, document images, and re-embedding.

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

client
    .upload_knowledge_urls(&UrlDocumentUploadRequest {
        knowledge_id: knowledge_id.clone(),
        upload_detail: vec![UrlDocument::new(
            "https://example.com/runbook",
            DocumentChunking::Heading,
        )],
    })
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

`RagDocumentUpload::from_bytes` deliberately has no path-based constructor. The caller controls file reads, size limits, encryption, tenant boundaries, and retention.

## MCP client

The `mcp` feature is a standalone Model Context Protocol client, separate from `McpTool`, which configures a hosted MCP tool inside a model request. Protocol framing, initialization, tools, resources, prompts, and Streamable HTTP transport are provided by the official Rust MCP SDK (`rmcp`).

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

Security defaults:

- Only absolute `http` and `https` endpoints are accepted.
- Authorization is explicit and redacted from `Debug` output.
- Redirects are disabled for the SDK-created HTTP client.
- SSE retry and automatic expired-session reinitialization are disabled by default.
- A caller-configured `reqwest::Client` can be injected for proxy, TLS, DNS, timeout, and policy control.

## Realtime WebSocket

The `realtime` feature provides typed client requests and typed server events over a bidirectional WebSocket. Audio and video are passed as caller-owned byte slices and encoded in memory.

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

The API also supports typed session tools, function-call outputs, response options, transcription sessions, client/server VAD, cancellation, audio commit/clear, video frames, and explicit connection close.

## Errors

`SdkError` is the public error envelope. Domain errors are explicit enums and remain matchable without parsing display strings.

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
        other => eprintln!("{other}"),
    }
}
```

The envelope distinguishes configuration, validation, transport, timeout, API, decode, stream, WebSocket, unsupported capability, agent, tool, Batch, RAG, and MCP failures. `ApiError` retains HTTP status, provider code, message, request ID, and raw response body.

## HTTP policy

`HttpConfig` controls request timeout, connect timeout, pool idle timeout, user agent, default headers, retry policy, and an optional caller-built `reqwest::Client`.

Retries are disabled by default. Enabling `RetryPolicy` is an explicit application decision; only the configured status codes and connection/timeout failures are retried.

## API coverage

The following table is an index of the public SDK operations. It is generated from the public client surface, not from an assumed provider feature set. Methods accepting `nextjson::Value` intentionally preserve compatibility with provider schemas that evolve faster than this crate.

| Area | Feature | Public operations |
| --- | --- | --- |
| Chat and streams | core; `tools` for ToolStream | `chat_completion`, `chat_completion_stream`, `chat_tool_stream`, `typed_chat_completion`, `typed_chat_completion_stream`, `typed_chat_tool_stream` |
| Async and vector APIs | core | `async_chat`, `async_result`, `embedding`, `rerank`, `tokenizer` |
| Images and video | `images`, `video` | `create_image`, `create_image_async`, `create_video` |
| Audio and voice | `audio` | `glm_4_voice`, `transcribe`, `speech`, `clone_voice`, `voices`, `delete_voice` |
| Hosted tools | `tools` | `web_search`, `read_web_page`, `moderate` |
| Files and document processing | `files` | `upload_file`, `files`, `file_content`, `delete_file`, `create_file_parse_task`, `file_parse_result`, `parse_file_sync`, `ocr`, `parse_layout` |
| Batch | `batch` | `create_batch`, `batches`, `batch`, `cancel_batch` |
| Official agents and assistants | `agents` | `official_agent`, `official_agent_stream`, `official_agent_async_result`, `official_agent_conversation`, `assistant`, `assistants`, `assistant_conversations` |
| Knowledge bases and retrieval | `rag` | `create_knowledge_base`, `knowledge_bases`, `knowledge_base`, `update_knowledge_base`, `delete_knowledge_base`, `knowledge_capacity`, `retrieve_knowledge`, `knowledge_documents`, `upload_knowledge_document`, `upload_knowledge_urls`, `knowledge_document`, `delete_knowledge_document`, `knowledge_document_images`, `reembed_knowledge_document`, `retrieval_agent_stream` |
| Protocol escape hatch | core | `request_json` on both `ZhipuClient` and `OpenAiCompatibleClient` |
| Standalone MCP | `mcp` | `McpClientConfig::connect`, plus typed tool, resource, prompt, and Streamable HTTP operations from `rmcp` |
| Realtime | `realtime` | `RealtimeConfig::connect`, typed requests/events, VAD, media buffers, function-call output, cancellation, and explicit close |

Use `ChatCompletionRequest` when a provider has released a field before it receives a typed builder. Use `request_json` only for relative paths on the configured provider base URL; it rejects absolute URLs and parent traversal segments.

> RustGLM exposes a provider-neutral local agent runtime, OpenAI-compatible client, generic `rmcp` protocol client, and video-capable Realtime session.

## Examples

The repository contains 36 runnable examples. Every current HTTP endpoint family has a focused example, while lifecycle examples deliberately group operations that are normally used together. `cargo check --all-targets --all-features` compiles all of them without contacting a provider.

### Chat, models, and vectors

| Example | Public API demonstrated |
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

| Example | Public API demonstrated |
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

| Example | Public API demonstrated |
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

| Example | Public API demonstrated |
| --- | --- |
| [`official_agent`](examples/official_agent.rs) | typed official Agent v1 invocation |
| [`official_agent_lifecycle`](examples/official_agent_lifecycle.rs) | Agent stream, async result, and conversation operations |
| [`assistants`](examples/assistants.rs) | Assistant invoke, list, and conversations |
| [`custom_agent`](examples/custom_agent.rs) | local agent runtime with an application tool |
| [`interactive_chat`](examples/interactive_chat.rs) | multi-turn runtime and optional semantic memory |
| [`mcp_client`](examples/mcp_client.rs) | MCP tools, resources, prompts, and close |
| [`realtime_audio_video`](examples/realtime_audio_video.rs) | Realtime PCM/WAV, optional JPEG frames, typed events |

Run an example with `cargo run --example <name> -- <arguments>`. The MCP client is opt-in, so use `cargo run --example mcp_client --features mcp -- <endpoint>`. Most Zhipu examples require `ZHIPU_API_KEY`; `openai_compatible` uses `OPENAI_COMPATIBLE_BASE_URL` and `OPENAI_COMPATIBLE_API_KEY`. Running examples can consume quota, create remote resources, or delete the explicitly named resource.

## CI and releases

The CI workflow verifies:

- formatting;
- Clippy with warnings denied;
- no-default, default, all-feature, and individual enterprise feature builds;
- tests and doctests;
- documentation with warnings denied;
- package construction from the committed lockfile;
- all-feature line coverage at or above 90%, with LCOV and text-summary artifacts.

The release workflow runs on `v*` tags. For a manual run, select the commit or branch to release in the Actions UI and enter `v<Cargo.toml version>` as the `tag` input. The workflow checks out that selected revision instead of assuming the tag already exists. It rejects a version mismatch, runs every release gate, builds the `.crate`, writes `SHA256SUMS`, and then creates the missing annotated tag. An existing tag is accepted only when it points at the verified commit. Finally, it optionally publishes to crates.io when `CARGO_REGISTRY_TOKEN` is configured and creates or updates the GitHub Release.

Release procedure:

```bash
# Update Cargo.toml and release notes first. Cargo.toml currently contains 1.0.0.
git tag -s v1.0.0 -m "RustGLM v1.0.0"
git push origin v1.0.0
```

Alternatively, run the `Release` workflow manually from the `main` branch with `tag` set to `v1.0.0`; no pre-existing tag is required. Tags use the plain `v1.0.0` form, not `RustGLM v1.0.0`.

No API key or registry token is stored in the repository. Configure `CARGO_REGISTRY_TOKEN` as a GitHub Actions secret only when crates.io publishing is required.

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

Coverage has a repository-local command and an enforced CI threshold:

```bash
cargo coverage       # summary plus the 90% line threshold
cargo coverage-lcov  # target/rustglm-lcov.info plus the same threshold
```

Latest verified working-tree snapshot (2026-07-24):

| Tests | Regions | Functions | Lines | Required lines |
| ---: | ---: | ---: | ---: | ---: |
| 94 passed, 2 live ignored | 92.72% | 88.33% | 94.02% | 90.00% |

The all-feature measurement includes every library module, including optional MCP and Realtime code. Knowledge/RAG line coverage is 96.63%; successful MCP protocol calls require an initialized peer, so its offline line coverage is 51.60%. The full per-module table, interpretation, and HTML command are in [docs/COVERAGE.md](docs/COVERAGE.md).

The coverage command runs offline unit and integration tests. It compiles all 36 examples but does not execute their `main` functions, and it does not run ignored live tests. Run credentialed checks deliberately:

```powershell
$env:ZHIPU_API_KEY = "key_id.secret"
cargo test --test live_zhipu -- --ignored --nocapture
cargo test --test live_realtime -- --ignored --nocapture
```

CI regenerates the values and publishes both `lcov.info` and `coverage-summary.txt`; use that artifact rather than treating the snapshot above as a permanent claim.

## License

Apache License 2.0. See [LICENSE](LICENSE).
