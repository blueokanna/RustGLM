# [RustGLM](README_zh.md) - Rust SDK for the Zhipu AI Open Platform

[![CI](https://github.com/blueokanna/rustglm/actions/workflows/ci.yml/badge.svg)](https://github.com/blueokanna/rustglm/actions/workflows/ci.yml)
[![Release](https://github.com/blueokanna/rustglm/actions/workflows/release.yml/badge.svg)](https://github.com/blueokanna/rustglm/actions/workflows/release.yml)
[![Crates.io](https://img.shields.io/crates/v/rustglm.svg)](https://crates.io/crates/rustglm)
[![Documentation](https://docs.rs/rustglm/badge.svg)](https://docs.rs/rustglm)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)

RustGLM is an unofficial, async Rust SDK for the Zhipu AI Open Platform. It provides typed GLM-5 requests, SSE and ToolStream assembly, bidirectional Realtime WebSocket sessions, Batch API operations, knowledge-base management, and an MCP client built on the official Rust MCP SDK.

The crate targets production backends. Network policy, persistence, credentials, retries, and client lifetime remain visible to the application.

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
    Glm52, ReasoningEffort, Thinking, TypedChatRequest, ZhipuClient,
};

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

Current typed model markers include:

- Text: `Glm52`, `Glm51`, `Glm5Turbo`, `Glm5`, `Glm47`, GLM-4.7 Flash variants, GLM-4.6, and selected GLM-4.5/4 Flash models.
- Vision: `Glm5vTurbo`, GLM-4.6V variants, `Glm4vFlash`, and GLM-4.1V Thinking variants.
- `ReasoningEffort` is capability-gated to `Glm52`.
- `Thinking`, tools, ToolStream, and vision input are exposed only on marker types that declare those capabilities.

`ChatCompletionRequest` remains available as a forward-compatible raw request when a newly released field has not yet received a typed builder.

## ToolStream

ToolStream combines fragmented SSE function-call deltas into complete typed calls while preserving text, reasoning, usage, and stream errors.

```rust,no_run
use futures_util::StreamExt;
use rustglm::{Glm52, ToolStreamEvent, TypedChatRequest, ZhipuClient};

# async fn run() -> rustglm::Result<()> {
let client = ZhipuClient::new("token")?;
let request = TypedChatRequest::<Glm52>::new()
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

## CI and releases

The CI workflow verifies:

- formatting;
- Clippy with warnings denied;
- no-default, default, all-feature, and individual enterprise feature builds;
- tests and doctests;
- documentation with warnings denied;
- package construction from the committed lockfile.

The release workflow runs on `v*` tags. It rejects a tag that does not exactly match `v<Cargo.toml version>`, reruns the release gates, builds the `.crate`, writes `SHA256SUMS`, optionally publishes to crates.io when `CARGO_REGISTRY_TOKEN` is configured, and creates the GitHub Release with those artifacts.

Release procedure:

```bash
# Update Cargo.toml and changelog/release notes first.
git tag -s v0.2.1 -m "RustGLM v0.2.1"
git push origin v0.2.1
```

No API key or registry token is stored in the repository. Configure `CARGO_REGISTRY_TOKEN` as a GitHub Actions secret only when crates.io publishing is required.

## Testing locally

```bash
cargo fmt --all -- --check
cargo test --all-targets --no-default-features
cargo test --all-targets
cargo test --all-targets --all-features
cargo clippy --all-targets --all-features -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --all-features --no-deps
cargo package --locked
```

Live tests are ignored by default because they require credentials and may incur charges.

## License

Apache License 2.0. See [LICENSE](LICENSE).
