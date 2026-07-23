use std::sync::Arc;

use async_stream::try_stream;
use async_trait::async_trait;
use reqwest::Method;
#[cfg(feature = "rag")]
use reqwest::header::{HeaderMap, HeaderValue};
#[cfg(any(feature = "audio", feature = "files"))]
use reqwest::multipart::{Form, Part};
use serde::Serialize;
use serde::de::DeserializeOwned;
#[cfg(any(
    feature = "agents",
    feature = "audio",
    feature = "batch",
    feature = "files",
    feature = "tools"
))]
use serde_json::Value;

#[cfg(feature = "video")]
use crate::VideoGenerationRequest;
#[cfg(feature = "agents")]
use crate::agent::official_agent_stream;
#[cfg(feature = "rag")]
use crate::agent::retrieval_agent_stream;
use crate::auth::AuthenticationProvider;
use crate::provider::{ChatProvider, ChatStream, ProviderCapabilities};
use crate::transport::Transport;
#[cfg(feature = "agents")]
use crate::{
    AgentAsyncResultRequest, AgentAsyncResultResponse, AgentConversationRequest,
    AgentConversationResponse, OfficialAgentRequest, OfficialAgentResponse, OfficialAgentStream,
};
use crate::{
    AsyncTaskResponse, AsyncTaskResult, ChatCompletionChunk, ChatCompletionRequest,
    ChatCompletionResponse, EmbeddingRequest, EmbeddingResponse, HttpConfig, RerankRequest,
    RerankResponse, Result, SdkError, TokenizerRequest, TokenizerResponse, ZhipuAuthentication,
};
#[cfg(feature = "batch")]
use crate::{BatchCreateRequest, BatchError, BatchList, BatchObject};
#[cfg(feature = "tools")]
use crate::{
    ChatModel, Ready, SupportsToolStream, ToolStream, TypedChatRequest, assemble_tool_stream,
};
#[cfg(not(feature = "tools"))]
use crate::{ChatModel, Ready, TypedChatRequest};
#[cfg(feature = "files")]
use crate::{DeleteResponse, FileList, FileObject, FileUploadRequest};
#[cfg(feature = "audio")]
use crate::{Glm4VoiceRequest, SpeechRequest, TranscriptionRequest, TranscriptionResponse};
#[cfg(feature = "images")]
use crate::{ImageGenerationRequest, ImageGenerationResponse};
#[cfg(feature = "rag")]
use crate::{RetrievalAgentRequest, RetrievalAgentStream};

pub const ZHIPU_BASE_URL: &str = "https://open.bigmodel.cn/api/paas/v4";
pub const ZHIPU_AGENT_BASE_URL: &str = "https://open.bigmodel.cn/api";

#[derive(Clone)]
pub struct ZhipuConfig {
    pub authentication: ZhipuAuthentication,
    pub base_url: String,
    pub agent_base_url: String,
    pub http: HttpConfig,
}

impl ZhipuConfig {
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            authentication: ZhipuAuthentication::auto(api_key),
            base_url: ZHIPU_BASE_URL.into(),
            agent_base_url: ZHIPU_AGENT_BASE_URL.into(),
            http: HttpConfig::default(),
        }
    }

    pub fn base_url(mut self, value: impl Into<String>) -> Self {
        self.base_url = value.into();
        self
    }

    pub fn authentication(mut self, value: ZhipuAuthentication) -> Self {
        self.authentication = value;
        self
    }

    pub fn agent_base_url(mut self, value: impl Into<String>) -> Self {
        self.agent_base_url = value.into();
        self
    }

    pub fn http(mut self, value: HttpConfig) -> Self {
        self.http = value;
        self
    }

    pub fn build(self) -> Result<ZhipuClient> {
        ZhipuClient::from_config(self)
    }
}

#[derive(Clone)]
pub struct ZhipuClient {
    pub(crate) transport: Arc<Transport>,
    pub(crate) agent_transport: Arc<Transport>,
}

impl ZhipuClient {
    pub fn new(api_key: impl Into<String>) -> Result<Self> {
        ZhipuConfig::new(api_key).build()
    }

    pub fn from_config(config: ZhipuConfig) -> Result<Self> {
        let authentication = AuthenticationProvider::zhipu(config.authentication)?;
        let transport =
            Transport::new(config.base_url, authentication.clone(), config.http.clone())?;
        let agent_transport = Transport::new(config.agent_base_url, authentication, config.http)?;
        Ok(Self {
            transport: Arc::new(transport),
            agent_transport: Arc::new(agent_transport),
        })
    }

    pub fn base_url(&self) -> &str {
        self.transport.base_url()
    }

    pub fn agent_base_url(&self) -> &str {
        self.agent_transport.base_url()
    }

    pub async fn chat_completion(
        &self,
        request: &ChatCompletionRequest,
    ) -> Result<ChatCompletionResponse> {
        validate_zhipu_chat(request)?;
        let mut request = request.clone();
        request.stream = false;
        self.transport.post_json("chat/completions", &request).await
    }

    pub async fn chat_completion_stream(
        &self,
        request: &ChatCompletionRequest,
    ) -> Result<ChatStream> {
        validate_zhipu_chat(request)?;
        let mut request = request.clone();
        request.stream = true;
        let response = self
            .transport
            .post_stream("chat/completions", &request)
            .await?;
        Ok(sse_stream(response))
    }

    #[cfg(feature = "tools")]
    pub async fn chat_tool_stream(&self, request: &ChatCompletionRequest) -> Result<ToolStream> {
        let mut request = request.clone();
        request.tool_stream = Some(true);
        let stream = self.chat_completion_stream(&request).await?;
        Ok(assemble_tool_stream(stream))
    }

    pub async fn typed_chat_completion<M: ChatModel>(
        &self,
        request: &TypedChatRequest<M, Ready>,
    ) -> Result<ChatCompletionResponse> {
        self.chat_completion(request.as_raw()).await
    }

    pub async fn typed_chat_completion_stream<M: ChatModel>(
        &self,
        request: &TypedChatRequest<M, Ready>,
    ) -> Result<ChatStream> {
        self.chat_completion_stream(request.as_raw()).await
    }

    #[cfg(feature = "tools")]
    pub async fn typed_chat_tool_stream<M: SupportsToolStream>(
        &self,
        request: &TypedChatRequest<M, Ready>,
    ) -> Result<ToolStream> {
        self.chat_tool_stream(request.as_raw()).await
    }

    #[cfg(feature = "audio")]
    pub async fn glm_4_voice(&self, request: &Glm4VoiceRequest) -> Result<ChatCompletionResponse> {
        self.chat_completion(request.as_chat_request()).await
    }

    #[cfg(feature = "agents")]
    pub async fn official_agent(
        &self,
        request: &OfficialAgentRequest,
    ) -> Result<OfficialAgentResponse> {
        validate_official_agent(request)?;
        let mut request = request.clone();
        request.stream = false;
        self.agent_transport.post_json("v1/agents", &request).await
    }

    #[cfg(feature = "agents")]
    pub async fn official_agent_stream(
        &self,
        request: &OfficialAgentRequest,
    ) -> Result<OfficialAgentStream> {
        validate_official_agent(request)?;
        let mut request = request.clone();
        request.stream = true;
        let response = self
            .agent_transport
            .post_stream("v1/agents", &request)
            .await?;
        Ok(official_agent_stream(response))
    }

    #[cfg(feature = "agents")]
    pub async fn official_agent_async_result(
        &self,
        request: &AgentAsyncResultRequest,
    ) -> Result<AgentAsyncResultResponse> {
        require_id(&request.async_id, "agent async id")?;
        require_id(&request.agent_id, "agent id")?;
        self.agent_transport
            .post_json("v1/agents/async-result", request)
            .await
    }

    #[cfg(feature = "agents")]
    pub async fn official_agent_conversation(
        &self,
        request: &AgentConversationRequest,
    ) -> Result<AgentConversationResponse> {
        require_id(&request.agent_id, "agent id")?;
        require_id(&request.conversation_id, "agent conversation id")?;
        self.agent_transport
            .post_json("v1/agents/conversation", request)
            .await
    }

    #[cfg(feature = "rag")]
    pub async fn retrieval_agent_stream(
        &self,
        request: &RetrievalAgentRequest,
        session_id: Option<&str>,
    ) -> Result<RetrievalAgentStream> {
        validate_retrieval_agent(request)?;
        let mut headers = HeaderMap::new();
        if let Some(session_id) = session_id {
            require_id(session_id, "agent session id")?;
            headers.insert(
                "x-session-id",
                HeaderValue::from_str(session_id).map_err(|_| {
                    SdkError::Validation("agent session id is not a valid header value".into())
                })?,
            );
        }
        let response = self
            .agent_transport
            .post_stream_with_headers("zrag/agent/chat", request, headers)
            .await?;
        Ok(retrieval_agent_stream(response))
    }

    pub async fn async_chat(&self, request: &ChatCompletionRequest) -> Result<AsyncTaskResponse> {
        validate_zhipu_chat(request)?;
        let mut request = request.clone();
        request.stream = false;
        self.transport
            .post_json("async/chat/completions", &request)
            .await
    }

    pub async fn async_result(&self, id: &str) -> Result<AsyncTaskResult> {
        require_id(id, "task id")?;
        self.transport
            .get_json(&format!("async-result/{}", encode_component(id)))
            .await
    }

    pub async fn embedding(&self, request: &EmbeddingRequest) -> Result<EmbeddingResponse> {
        if request.model.trim().is_empty() {
            return Err(SdkError::Validation(
                "embedding model cannot be empty".into(),
            ));
        }
        self.transport.post_json("embeddings", request).await
    }

    pub async fn rerank(&self, request: &RerankRequest) -> Result<RerankResponse> {
        if request.model.trim().is_empty()
            || request.query.trim().is_empty()
            || request.documents.is_empty()
        {
            return Err(SdkError::Validation(
                "rerank requires model, query, and at least one document".into(),
            ));
        }
        self.transport.post_json("rerank", request).await
    }

    pub async fn tokenizer(&self, request: &TokenizerRequest) -> Result<TokenizerResponse> {
        if request.model.trim().is_empty() || request.messages.is_empty() {
            return Err(SdkError::Validation(
                "tokenizer requires model and at least one message".into(),
            ));
        }
        self.transport.post_json("tokenizer", request).await
    }

    #[cfg(feature = "images")]
    pub async fn create_image(
        &self,
        request: &ImageGenerationRequest,
    ) -> Result<ImageGenerationResponse> {
        validate_image(request)?;
        self.transport
            .post_json("images/generations", request)
            .await
    }

    #[cfg(feature = "images")]
    pub async fn create_image_async(
        &self,
        request: &ImageGenerationRequest,
    ) -> Result<AsyncTaskResponse> {
        validate_image(request)?;
        self.transport
            .post_json("async/images/generations", request)
            .await
    }

    #[cfg(feature = "video")]
    pub async fn create_video(
        &self,
        request: &VideoGenerationRequest,
    ) -> Result<AsyncTaskResponse> {
        if request.model.trim().is_empty() {
            return Err(SdkError::Validation("video model cannot be empty".into()));
        }
        if request
            .prompt
            .as_deref()
            .unwrap_or_default()
            .trim()
            .is_empty()
            && request.image_url.is_none()
        {
            return Err(SdkError::Validation(
                "video generation requires prompt or image_url".into(),
            ));
        }
        self.transport
            .post_json("videos/generations", request)
            .await
    }

    #[cfg(feature = "audio")]
    pub async fn transcribe(&self, request: TranscriptionRequest) -> Result<TranscriptionResponse> {
        if request.model.trim().is_empty()
            || request.file.is_empty()
            || request.file_name.trim().is_empty()
        {
            return Err(SdkError::Validation(
                "transcription requires model, file name, and file bytes".into(),
            ));
        }
        let mut part = Part::bytes(request.file).file_name(request.file_name);
        if let Some(mime_type) = request.mime_type {
            part = part
                .mime_str(&mime_type)
                .map_err(|error| SdkError::Validation(error.to_string().into()))?;
        }
        let mut form = Form::new().part("file", part).text("model", request.model);
        if let Some(prompt) = request.prompt {
            form = form.text("prompt", prompt);
        }
        if !request.hotwords.is_empty() {
            form = form.text(
                "hotwords",
                serde_json::to_string(&request.hotwords)
                    .map_err(|error| SdkError::Validation(error.to_string().into()))?,
            );
        }
        if let Some(request_id) = request.request_id {
            form = form.text("request_id", request_id);
        }
        if let Some(user_id) = request.user_id {
            form = form.text("user_id", user_id);
        }
        self.transport
            .post_multipart("audio/transcriptions", form)
            .await
    }

    #[cfg(feature = "audio")]
    pub async fn speech(&self, request: &SpeechRequest) -> Result<Vec<u8>> {
        if request.model.trim().is_empty()
            || request.input.trim().is_empty()
            || request.voice.trim().is_empty()
        {
            return Err(SdkError::Validation(
                "speech requires model, input, and voice".into(),
            ));
        }
        self.transport
            .post_binary("audio/speech", request, "audio/*")
            .await
    }

    #[cfg(feature = "audio")]
    pub async fn clone_voice(&self, request: &Value) -> Result<Value> {
        self.transport.post_json("voice/clone", request).await
    }

    #[cfg(feature = "audio")]
    pub async fn voices(&self) -> Result<Value> {
        self.transport.get_json("voice/list").await
    }

    #[cfg(feature = "audio")]
    pub async fn delete_voice(&self, request: &Value) -> Result<Value> {
        self.transport.post_json("voice/delete", request).await
    }

    #[cfg(feature = "tools")]
    pub async fn web_search(&self, request: &Value) -> Result<Value> {
        self.transport.post_json("web_search", request).await
    }

    #[cfg(feature = "tools")]
    pub async fn read_web_page(&self, request: &Value) -> Result<Value> {
        self.transport.post_json("reader", request).await
    }

    #[cfg(feature = "tools")]
    pub async fn moderate(&self, request: &Value) -> Result<Value> {
        self.transport.post_json("moderations", request).await
    }

    #[cfg(feature = "files")]
    pub async fn parse_layout(&self, request: &Value) -> Result<Value> {
        self.transport.post_json("layout_parsing", request).await
    }

    #[cfg(feature = "files")]
    pub async fn upload_file(&self, request: FileUploadRequest) -> Result<FileObject> {
        if request.file.is_empty()
            || request.file_name.trim().is_empty()
            || request.purpose.trim().is_empty()
        {
            return Err(SdkError::Validation(
                "file upload requires file name, bytes, and purpose".into(),
            ));
        }
        let mut part = Part::bytes(request.file).file_name(request.file_name);
        if let Some(mime_type) = request.mime_type {
            part = part
                .mime_str(&mime_type)
                .map_err(|error| SdkError::Validation(error.to_string().into()))?;
        }
        let form = Form::new()
            .part("file", part)
            .text("purpose", request.purpose);
        self.transport.post_multipart("files", form).await
    }

    #[cfg(feature = "files")]
    pub async fn files(&self, purpose: Option<&str>, limit: Option<u32>) -> Result<FileList> {
        let mut query = Vec::new();
        if let Some(purpose) = purpose {
            query.push(format!("purpose={}", encode_component(purpose)));
        }
        if let Some(limit) = limit {
            query.push(format!("limit={limit}"));
        }
        let path = if query.is_empty() {
            "files".to_owned()
        } else {
            format!("files?{}", query.join("&"))
        };
        self.transport.get_json(&path).await
    }

    #[cfg(feature = "files")]
    pub async fn delete_file(&self, file_id: &str) -> Result<DeleteResponse> {
        require_id(file_id, "file id")?;
        self.transport
            .delete_json(&format!("files/{}", encode_component(file_id)))
            .await
    }

    #[cfg(feature = "files")]
    pub async fn file_content(&self, file_id: &str) -> Result<Vec<u8>> {
        require_id(file_id, "file id")?;
        self.transport
            .get_binary(&format!("files/{}/content", encode_component(file_id)))
            .await
    }

    #[cfg(feature = "files")]
    pub async fn create_file_parse_task(&self, request: &Value) -> Result<Value> {
        self.transport
            .post_json("files/parser/create", request)
            .await
    }

    #[cfg(feature = "files")]
    pub async fn file_parse_result(&self, task_id: &str, format_type: &str) -> Result<Value> {
        require_id(task_id, "parser task id")?;
        require_id(format_type, "parser format type")?;
        self.transport
            .get_json(&format!(
                "files/parser/result/{}/{}",
                encode_component(task_id),
                encode_component(format_type)
            ))
            .await
    }

    #[cfg(feature = "files")]
    pub async fn parse_file_sync(&self, request: &Value) -> Result<Value> {
        self.transport.post_json("files/parser/sync", request).await
    }

    #[cfg(feature = "files")]
    pub async fn ocr(&self, request: &Value) -> Result<Value> {
        self.transport.post_json("files/ocr", request).await
    }

    #[cfg(feature = "batch")]
    pub async fn create_batch(&self, request: &BatchCreateRequest) -> Result<BatchObject> {
        if request.input_file_id.trim().is_empty() || request.endpoint.trim().is_empty() {
            return Err(BatchError::MissingCreateFields.into());
        }
        self.transport.post_json("batches", request).await
    }

    #[cfg(feature = "batch")]
    pub async fn batches(&self, limit: Option<u32>, after: Option<&str>) -> Result<BatchList> {
        let mut query = Vec::new();
        if let Some(limit) = limit {
            if !(1..=100).contains(&limit) {
                return Err(BatchError::InvalidLimit(limit).into());
            }
            query.push(format!("limit={limit}"));
        }
        if let Some(after) = after {
            query.push(format!("after={}", encode_component(after)));
        }
        let path = if query.is_empty() {
            "batches".to_owned()
        } else {
            format!("batches?{}", query.join("&"))
        };
        self.transport.get_json(&path).await
    }

    #[cfg(feature = "batch")]
    pub async fn batch(&self, batch_id: &str) -> Result<BatchObject> {
        require_id(batch_id, "batch id")?;
        self.transport
            .get_json(&format!("batches/{}", encode_component(batch_id)))
            .await
    }

    #[cfg(feature = "batch")]
    pub async fn cancel_batch(&self, batch_id: &str) -> Result<BatchObject> {
        require_id(batch_id, "batch id")?;
        self.transport
            .post_json(
                &format!("batches/{}/cancel", encode_component(batch_id)),
                &Value::Null,
            )
            .await
    }

    #[cfg(feature = "agents")]
    pub async fn assistant(&self, request: &Value) -> Result<Value> {
        self.transport.post_json("assistant", request).await
    }

    #[cfg(feature = "agents")]
    pub async fn assistants(&self, request: &Value) -> Result<Value> {
        self.transport.post_json("assistant/list", request).await
    }

    #[cfg(feature = "agents")]
    pub async fn assistant_conversations(&self, request: &Value) -> Result<Value> {
        self.transport
            .post_json("assistant/conversation/list", request)
            .await
    }

    pub async fn request_json<T, R>(
        &self,
        method: Method,
        path: &str,
        body: Option<&T>,
    ) -> Result<R>
    where
        T: Serialize + ?Sized,
        R: DeserializeOwned,
    {
        self.transport.request_json(method, path, body).await
    }
}

#[derive(Clone)]
pub struct OpenAiCompatibleConfig {
    pub name: String,
    pub api_key: String,
    pub base_url: String,
    pub chat_path: String,
    pub http: HttpConfig,
}

impl OpenAiCompatibleConfig {
    pub fn new(
        name: impl Into<String>,
        api_key: impl Into<String>,
        base_url: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            api_key: api_key.into(),
            base_url: base_url.into(),
            chat_path: "chat/completions".into(),
            http: HttpConfig::default(),
        }
    }

    pub fn chat_path(mut self, value: impl Into<String>) -> Self {
        self.chat_path = value.into();
        self
    }

    pub fn http(mut self, value: HttpConfig) -> Self {
        self.http = value;
        self
    }

    pub fn build(self) -> Result<OpenAiCompatibleClient> {
        OpenAiCompatibleClient::from_config(self)
    }
}

#[derive(Clone)]
pub struct OpenAiCompatibleClient {
    name: String,
    chat_path: String,
    transport: Arc<Transport>,
}

impl OpenAiCompatibleClient {
    pub fn from_config(config: OpenAiCompatibleConfig) -> Result<Self> {
        if config.name.trim().is_empty() || config.chat_path.trim().is_empty() {
            return Err(SdkError::Configuration(
                "provider name and chat path cannot be empty".into(),
            ));
        }
        let authentication = AuthenticationProvider::bearer(config.api_key)?;
        let transport = Transport::new(config.base_url, authentication, config.http)?;
        Ok(Self {
            name: config.name,
            chat_path: config.chat_path,
            transport: Arc::new(transport),
        })
    }

    pub async fn chat_completion(
        &self,
        request: &ChatCompletionRequest,
    ) -> Result<ChatCompletionResponse> {
        validate_openai_chat(request)?;
        let mut request = request.clone();
        request.stream = false;
        self.transport.post_json(&self.chat_path, &request).await
    }

    pub async fn chat_completion_stream(
        &self,
        request: &ChatCompletionRequest,
    ) -> Result<ChatStream> {
        validate_openai_chat(request)?;
        let mut request = request.clone();
        request.stream = true;
        let response = self
            .transport
            .post_stream(&self.chat_path, &request)
            .await?;
        Ok(sse_stream(response))
    }

    pub async fn request_json<T, R>(
        &self,
        method: Method,
        path: &str,
        body: Option<&T>,
    ) -> Result<R>
    where
        T: Serialize + ?Sized,
        R: DeserializeOwned,
    {
        self.transport.request_json(method, path, body).await
    }
}

#[async_trait]
impl ChatProvider for ZhipuClient {
    fn name(&self) -> &str {
        "zhipu"
    }

    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities::zhipu()
    }

    async fn complete(&self, request: ChatCompletionRequest) -> Result<ChatCompletionResponse> {
        self.chat_completion(&request).await
    }

    async fn stream(&self, request: ChatCompletionRequest) -> Result<ChatStream> {
        self.chat_completion_stream(&request).await
    }
}

#[async_trait]
impl ChatProvider for OpenAiCompatibleClient {
    fn name(&self) -> &str {
        &self.name
    }

    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities::openai_compatible()
    }

    async fn complete(&self, request: ChatCompletionRequest) -> Result<ChatCompletionResponse> {
        self.chat_completion(&request).await
    }

    async fn stream(&self, request: ChatCompletionRequest) -> Result<ChatStream> {
        self.chat_completion_stream(&request).await
    }
}

#[derive(Default)]
struct SseDecoder {
    buffer: Vec<u8>,
    event: Vec<u8>,
    done: bool,
}

impl SseDecoder {
    fn push(&mut self, bytes: &[u8]) -> Result<Vec<ChatCompletionChunk>> {
        if self.done {
            return Ok(Vec::new());
        }
        self.buffer.extend_from_slice(bytes);
        self.drain(false)
    }

    fn finish(&mut self) -> Result<Vec<ChatCompletionChunk>> {
        self.drain(true)
    }

    fn drain(&mut self, finish: bool) -> Result<Vec<ChatCompletionChunk>> {
        let mut chunks = Vec::new();
        while let Some(position) = self.buffer.iter().position(|byte| *byte == b'\n') {
            let mut line: Vec<u8> = self.buffer.drain(..=position).collect();
            line.pop();
            if line.last() == Some(&b'\r') {
                line.pop();
            }
            self.consume_line(&line, &mut chunks)?;
        }
        if finish {
            if !self.buffer.is_empty() {
                let line = std::mem::take(&mut self.buffer);
                self.consume_line(&line, &mut chunks)?;
            }
            self.consume_event(&mut chunks)?;
        }
        Ok(chunks)
    }

    fn consume_line(&mut self, line: &[u8], chunks: &mut Vec<ChatCompletionChunk>) -> Result<()> {
        if line.is_empty() {
            return self.consume_event(chunks);
        }
        if line.starts_with(b"data:") {
            let mut data = &line[5..];
            if data.first() == Some(&b' ') {
                data = &data[1..];
            }
            if !self.event.is_empty() {
                self.event.push(b'\n');
            }
            self.event.extend_from_slice(data);
        }
        Ok(())
    }

    fn consume_event(&mut self, chunks: &mut Vec<ChatCompletionChunk>) -> Result<()> {
        if self.event.is_empty() {
            return Ok(());
        }
        let event = std::mem::take(&mut self.event);
        if event == b"[DONE]" {
            self.done = true;
            return Ok(());
        }
        let chunk = serde_json::from_slice(&event).map_err(|error| {
            SdkError::Stream(format!("{}: {}", error, String::from_utf8_lossy(&event)).into())
        })?;
        chunks.push(chunk);
        Ok(())
    }
}

fn sse_stream(response: reqwest::Response) -> ChatStream {
    let stream = try_stream! {
        let mut response = response;
        let mut decoder = SseDecoder::default();
        while let Some(bytes) = response.chunk().await? {
            for chunk in decoder.push(&bytes)? {
                yield chunk;
            }
        }
        for chunk in decoder.finish()? {
            yield chunk;
        }
    };
    Box::pin(stream)
}

fn validate_zhipu_chat(request: &ChatCompletionRequest) -> Result<()> {
    validate_chat(request, 1.0, 0.01)?;
    if request
        .max_tokens
        .is_some_and(|value| value == 0 || value > 131_072)
    {
        return Err(SdkError::Validation(
            "Zhipu max_tokens must be between 1 and 131072".into(),
        ));
    }
    validate_optional_id(request.request_id.as_deref(), "request_id", 6, 64)?;
    validate_optional_id(request.user_id.as_deref(), "user_id", 6, 128)?;
    Ok(())
}

#[cfg(feature = "agents")]
fn validate_official_agent(request: &OfficialAgentRequest) -> Result<()> {
    if request.agent_id.trim().is_empty() || request.messages.is_empty() {
        return Err(SdkError::Validation(
            "official agent requires agent_id and at least one message".into(),
        ));
    }
    Ok(())
}

#[cfg(feature = "rag")]
fn validate_retrieval_agent(request: &RetrievalAgentRequest) -> Result<()> {
    if request.messages.is_empty()
        || request.retrieval.know_ids.is_empty()
        || request.model.trim().is_empty()
        || request.max_steps == 0
        || request.retrieval.top_k == 0
        || request.retrieval.top_n == 0
        || !(0.0..=2.0).contains(&request.temperature)
        || !(0.0..=1.0).contains(&request.retrieval.similarity_threshold)
    {
        return Err(SdkError::Validation(
            "retrieval agent request contains invalid messages, model, steps, or retrieval settings"
                .into(),
        ));
    }
    Ok(())
}

fn validate_openai_chat(request: &ChatCompletionRequest) -> Result<()> {
    validate_chat(request, 2.0, 0.0)
}

fn validate_chat(
    request: &ChatCompletionRequest,
    temperature_max: f32,
    top_p_min: f32,
) -> Result<()> {
    if request.model.trim().is_empty() {
        return Err(SdkError::Validation("chat model cannot be empty".into()));
    }
    if request.messages.is_empty() {
        return Err(SdkError::Validation(
            "chat requires at least one message".into(),
        ));
    }
    if request
        .temperature
        .is_some_and(|value| !(0.0..=temperature_max).contains(&value))
    {
        return Err(SdkError::Validation(
            format!("temperature must be between 0 and {temperature_max}").into(),
        ));
    }
    if request
        .top_p
        .is_some_and(|value| !(top_p_min..=1.0).contains(&value))
    {
        return Err(SdkError::Validation(
            format!("top_p must be between {top_p_min} and 1").into(),
        ));
    }
    if request.stop.as_ref().is_some_and(|value| value.len() > 4) {
        return Err(SdkError::Validation(
            "stop cannot contain more than four values".into(),
        ));
    }
    Ok(())
}

fn validate_optional_id(value: Option<&str>, name: &str, min: usize, max: usize) -> Result<()> {
    if let Some(value) = value {
        let length = value.chars().count();
        if !(min..=max).contains(&length) {
            return Err(SdkError::Validation(
                format!("{name} length must be between {min} and {max}").into(),
            ));
        }
    }
    Ok(())
}

#[cfg(feature = "images")]
fn validate_image(request: &ImageGenerationRequest) -> Result<()> {
    if request.model.trim().is_empty() || request.prompt.trim().is_empty() {
        return Err(SdkError::Validation(
            "image generation requires model and prompt".into(),
        ));
    }
    Ok(())
}

fn require_id(value: &str, name: &str) -> Result<()> {
    if value.trim().is_empty() {
        return Err(SdkError::Validation(
            format!("{name} cannot be empty").into(),
        ));
    }
    Ok(())
}

pub(crate) fn encode_component(value: &str) -> String {
    let mut output = String::with_capacity(value.len());
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~') {
            output.push(byte as char);
        } else {
            output.push('%');
            output.push_str(&format!("{byte:02X}"));
        }
    }
    output
}

#[cfg(all(
    test,
    feature = "agents",
    feature = "audio",
    feature = "batch",
    feature = "files",
    feature = "images",
    feature = "rag",
    feature = "tools",
    feature = "video"
))]
mod tests {
    use std::time::Duration;

    use futures_util::StreamExt;
    use serde_json::json;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    use super::*;
    use crate::{
        ContentPart, FunctionDefinition, Glm52, MessageRole, Tool, ToolStreamEvent,
        TypedChatRequest,
    };

    struct MockResponse {
        status: &'static str,
        content_type: &'static str,
        body: &'static str,
    }

    impl MockResponse {
        fn json(body: &'static str) -> Self {
            Self {
                status: "200 OK",
                content_type: "application/json",
                body,
            }
        }

        fn binary(body: &'static str) -> Self {
            Self {
                status: "200 OK",
                content_type: "application/octet-stream",
                body,
            }
        }

        fn sse(body: &'static str) -> Self {
            Self {
                status: "200 OK",
                content_type: "text/event-stream",
                body,
            }
        }
    }

    async fn mock_server(
        responses: Vec<MockResponse>,
    ) -> (String, tokio::task::JoinHandle<Vec<String>>) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let mut requests = Vec::new();
            for response in responses {
                let (mut socket, _) = listener.accept().await.unwrap();
                let mut request = Vec::new();
                let mut buffer = [0u8; 4096];
                let mut expected = None;
                loop {
                    let read = socket.read(&mut buffer).await.unwrap();
                    if read == 0 {
                        break;
                    }
                    request.extend_from_slice(&buffer[..read]);
                    if let Some(end) = request
                        .windows(4)
                        .position(|part| part == b"\r\n\r\n")
                        .filter(|_| expected.is_none())
                    {
                        let headers = String::from_utf8_lossy(&request[..end]);
                        let content_length = headers
                            .lines()
                            .find_map(|line| {
                                let (name, value) = line.split_once(':')?;
                                name.eq_ignore_ascii_case("content-length")
                                    .then(|| value.trim().parse::<usize>().ok())
                                    .flatten()
                            })
                            .unwrap_or(0);
                        expected = Some(end + 4 + content_length);
                    }
                    if expected.is_some_and(|length| request.len() >= length) {
                        break;
                    }
                }
                requests.push(String::from_utf8_lossy(&request).into_owned());
                let output = format!(
                    "HTTP/1.1 {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    response.status,
                    response.content_type,
                    response.body.len(),
                    response.body
                );
                socket.write_all(output.as_bytes()).await.unwrap();
            }
            requests
        });
        (format!("http://{address}"), server)
    }

    fn valid_chat() -> ChatCompletionRequest {
        ChatCompletionRequest::new("glm-test").message(crate::ChatMessage::user("hello"))
    }

    fn assert_validation<T>(result: Result<T>, text: &str) {
        match result {
            Err(SdkError::Validation(message)) => assert!(message.contains(text), "{message}"),
            _ => panic!("expected validation error containing {text}"),
        }
    }

    #[test]
    fn multimodal_request_matches_wire_format() {
        let request =
            ChatCompletionRequest::new("glm-5v-turbo").message(crate::ChatMessage::multimodal(
                MessageRole::User,
                vec![
                    ContentPart::image_url("https://example.com/image.png"),
                    ContentPart::text("describe"),
                ],
            ));
        let value = serde_json::to_value(request).unwrap();
        assert_eq!(value["messages"][0]["content"][0]["type"], "image_url");
        assert_eq!(
            value["messages"][0]["content"][0]["image_url"]["url"],
            "https://example.com/image.png"
        );
    }

    #[test]
    fn function_tool_matches_wire_format() {
        let tool = Tool::function(FunctionDefinition::new(
            "weather",
            json!({"type":"object","properties":{"city":{"type":"string"}}}),
        ));
        let value = serde_json::to_value(tool).unwrap();
        assert_eq!(value["type"], "function");
        assert_eq!(value["function"]["name"], "weather");
    }

    #[test]
    fn sse_handles_split_utf8_and_event_frames() {
        let json = serde_json::to_vec(&json!({
            "id":"1",
            "model":"glm-5.2",
            "choices":[{"index":0,"delta":{"content":"你好"},"finish_reason":null}]
        }))
        .unwrap();
        let mut frame = b"data: ".to_vec();
        frame.extend(json);
        frame.extend_from_slice(b"\n\ndata: [DONE]\n\n");
        let split = frame.iter().position(|value| *value >= 0x80).unwrap() + 1;
        let mut decoder = SseDecoder::default();
        assert!(decoder.push(&frame[..split]).unwrap().is_empty());
        let chunks = decoder.push(&frame[split..]).unwrap();
        assert_eq!(chunks.len(), 1);
        assert_eq!(
            chunks[0].choices[0]
                .delta
                .content
                .as_ref()
                .and_then(crate::ResponseContent::as_text),
            Some("你好")
        );
    }

    #[test]
    fn rejects_unsafe_raw_paths() {
        assert!(
            super::super::transport::Transport::new(
                ZHIPU_BASE_URL.into(),
                AuthenticationProvider::bearer("key").unwrap(),
                HttpConfig::default()
            )
            .unwrap()
            .url_for_test("../secret")
            .is_err()
        );
    }

    #[tokio::test]
    async fn sends_bearer_auth_and_decodes_chat_response() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut request = Vec::new();
            let mut buffer = [0u8; 2048];
            loop {
                let read = socket.read(&mut buffer).await.unwrap();
                if read == 0 {
                    break;
                }
                request.extend_from_slice(&buffer[..read]);
                if request.windows(4).any(|value| value == b"\r\n\r\n") {
                    break;
                }
            }
            let request = String::from_utf8(request).unwrap();
            assert!(request.starts_with("POST /chat/completions HTTP/1.1"));
            assert!(
                request
                    .to_ascii_lowercase()
                    .contains("authorization: bearer test-key")
            );
            let body = r#"{"id":"id-1","model":"glm-5.2","choices":[{"index":0,"message":{"role":"assistant","content":"ok"},"finish_reason":"stop"}],"usage":{"prompt_tokens":1,"completion_tokens":1,"total_tokens":2}}"#;
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            socket.write_all(response.as_bytes()).await.unwrap();
        });
        let client = ZhipuConfig::new("test-key")
            .base_url(format!("http://{address}"))
            .build()
            .unwrap();
        let request = ChatCompletionRequest::new("glm-5.2").message(crate::ChatMessage::user("hi"));
        let response = client.chat_completion(&request).await.unwrap();
        assert_eq!(response.text(), Some("ok"));
        server.await.unwrap();
    }

    #[test]
    fn validates_chat_image_ids_and_paths() {
        let mut request = valid_chat();
        request.model = " ".into();
        assert_validation(validate_zhipu_chat(&request), "model");

        let mut request = ChatCompletionRequest::new("model");
        assert_validation(validate_zhipu_chat(&request), "message");
        request.messages.push(crate::ChatMessage::user("x"));
        request.temperature = Some(1.1);
        assert_validation(validate_zhipu_chat(&request), "temperature");
        request.temperature = Some(f32::NAN);
        assert_validation(validate_zhipu_chat(&request), "temperature");
        request.temperature = None;
        request.top_p = Some(0.0);
        assert_validation(validate_zhipu_chat(&request), "top_p");
        request.top_p = Some(1.1);
        assert_validation(validate_openai_chat(&request), "top_p");
        request.top_p = None;
        request.stop = Some(vec![
            "a".into(),
            "b".into(),
            "c".into(),
            "d".into(),
            "e".into(),
        ]);
        assert_validation(validate_zhipu_chat(&request), "stop");
        request.stop = None;
        request.max_tokens = Some(0);
        assert_validation(validate_zhipu_chat(&request), "max_tokens");
        request.max_tokens = Some(131_073);
        assert_validation(validate_zhipu_chat(&request), "max_tokens");
        request.max_tokens = Some(1);
        request.request_id = Some("short".into());
        assert_validation(validate_zhipu_chat(&request), "request_id");
        request.request_id = Some("a".repeat(65));
        assert_validation(validate_zhipu_chat(&request), "request_id");
        request.request_id = Some("valid-id".into());
        request.user_id = Some("short".into());
        assert_validation(validate_zhipu_chat(&request), "user_id");
        request.user_id = Some("a".repeat(129));
        assert_validation(validate_zhipu_chat(&request), "user_id");
        request.user_id = Some("valid-user".into());
        assert!(validate_zhipu_chat(&request).is_ok());

        let image = ImageGenerationRequest::default();
        assert_validation(validate_image(&image), "model and prompt");
        assert_validation(require_id(" ", "item"), "item");
        assert_eq!(encode_component("a b/c?中"), "a%20b%2Fc%3F%E4%B8%AD");
    }

    #[test]
    fn sse_handles_completion_errors_and_ignored_lines() {
        let mut decoder = SseDecoder::default();
        let chunks = decoder
            .push(b": ping\r\nevent: message\r\ndata: {\"id\":\"one\",\r\ndata: \"choices\":[]}\r\n\r\n")
            .unwrap();
        assert_eq!(chunks[0].id, "one");
        assert!(decoder.push(b"data: [DONE]\n\n").unwrap().is_empty());
        assert!(decoder.push(b"data: {bad}\n\n").unwrap().is_empty());
        assert!(decoder.finish().unwrap().is_empty());

        let mut decoder = SseDecoder::default();
        let chunks = decoder
            .push(b"data:{\"id\":\"two\",\"choices\":[]}")
            .unwrap();
        assert!(chunks.is_empty());
        assert_eq!(decoder.finish().unwrap()[0].id, "two");

        let mut decoder = SseDecoder::default();
        let error = decoder.push(b"data: {bad}\n\n").unwrap_err();
        assert!(matches!(error, SdkError::Stream(_)));
    }

    #[tokio::test]
    async fn streams_through_zhipu_and_compatible_clients() {
        let event = "data: {\"id\":\"chunk\",\"choices\":[]}\n\ndata: [DONE]\n\n";
        let (base_url, server) =
            mock_server(vec![MockResponse::sse(event), MockResponse::sse(event)]).await;
        let zhipu = ZhipuConfig::new("key").base_url(&base_url).build().unwrap();
        let mut first = zhipu.chat_completion_stream(&valid_chat()).await.unwrap();
        assert_eq!(first.next().await.unwrap().unwrap().id, "chunk");
        assert!(first.next().await.is_none());

        let compatible = OpenAiCompatibleConfig::new("custom", "key", &base_url)
            .chat_path("v1/chat")
            .http(HttpConfig::default())
            .build()
            .unwrap();
        let mut second = ChatProvider::stream(&compatible, valid_chat())
            .await
            .unwrap();
        assert_eq!(second.next().await.unwrap().unwrap().id, "chunk");
        assert_eq!(compatible.name(), "custom");
        assert!(compatible.capabilities().streaming);
        let requests = server.await.unwrap();
        assert!(requests[0].starts_with("POST /chat/completions "));
        assert!(requests[0].contains("\"stream\":true"));
        assert!(requests[1].starts_with("POST /v1/chat "));
    }

    #[tokio::test]
    async fn typed_chat_methods_and_tool_stream_use_checked_model() {
        let tool_events = concat!(
            "data: {\"id\":\"one\",\"choices\":[{\"index\":0,\"delta\":{\"tool_calls\":[{\"index\":0,\"id\":\"call-1\",\"type\":\"function\",\"function\":{\"name\":\"weather\",\"arguments\":\"{\\\"city\\\":\"}}]},\"finish_reason\":null}]}\n\n",
            "data: {\"id\":\"two\",\"choices\":[{\"index\":0,\"delta\":{\"tool_calls\":[{\"index\":0,\"function\":{\"arguments\":\"\\\"Beijing\\\"}\"}}]},\"finish_reason\":\"tool_calls\"}]}\n\n",
            "data: [DONE]\n\n"
        );
        let (base_url, server) = mock_server(vec![
            MockResponse::json(
                r#"{"id":"response","model":"glm-5.2","choices":[{"message":{"content":"ok"}}]}"#,
            ),
            MockResponse::sse("data: {\"id\":\"chunk\",\"choices\":[]}\n\ndata: [DONE]\n\n"),
            MockResponse::sse(tool_events),
        ])
        .await;
        let client = ZhipuConfig::new("key").base_url(base_url).build().unwrap();
        let request = TypedChatRequest::<Glm52>::new().user("hello");

        assert_eq!(
            client.typed_chat_completion(&request).await.unwrap().text(),
            Some("ok")
        );
        let mut stream = client.typed_chat_completion_stream(&request).await.unwrap();
        assert_eq!(stream.next().await.unwrap().unwrap().id, "chunk");

        let mut stream = client.typed_chat_tool_stream(&request).await.unwrap();
        let mut completed = None;
        while let Some(event) = stream.next().await {
            if let ToolStreamEvent::ToolCallCompleted(call) = event.unwrap() {
                completed = Some(call);
            }
        }
        let completed = completed.unwrap();
        assert_eq!(completed.name, "weather");
        assert_eq!(completed.arguments, r#"{"city":"Beijing"}"#);

        let requests = server.await.unwrap();
        assert!(
            requests
                .iter()
                .all(|request| request.contains("\"model\":\"glm-5.2\""))
        );
        assert!(requests[2].contains("\"tool_stream\":true"));
    }

    #[tokio::test]
    async fn calls_voice_and_strongly_typed_agent_endpoints() {
        let (base_url, server) = mock_server(vec![
            MockResponse::json(
                r#"{"model":"glm-4-voice","choices":[{"message":{"role":"assistant","content":"heard","audio":{"data":"AQI=","expires_at":1749187238}}}]}"#,
            ),
            MockResponse::json(
                r#"{"id":"agent-1","agent_id":"general_translation","choices":[]}"#,
            ),
            MockResponse::sse(
                "data: {\"id\":\"agent-stream\",\"choices\":[]}\n\ndata: [DONE]\n\n",
            ),
            MockResponse::json(
                r#"{"agent_id":"agent","async_id":"async","status":"success","choices":[]}"#,
            ),
            MockResponse::json(
                r#"{"agent_id":"slides_glm_agent","conversation_id":"conversation","choices":[]}"#,
            ),
            MockResponse::sse(
                "data: {\"type\":\"session_created\",\"sessionId\":\"session-1\"}\n\ndata: {\"type\":\"done\",\"messageId\":\"message-1\"}\n\ndata: [DONE]\n\n",
            ),
        ])
        .await;
        let client = ZhipuConfig::new("key")
            .base_url(&base_url)
            .agent_base_url(&base_url)
            .build()
            .unwrap();
        assert_eq!(client.agent_base_url(), base_url);

        let voice = client
            .glm_4_voice(&Glm4VoiceRequest::from_wav("repeat", b"RIFF").unwrap())
            .await
            .unwrap();
        assert_eq!(voice.text(), Some("heard"));
        assert_eq!(voice.audio_bytes().unwrap(), Some(vec![1, 2]));

        let request = OfficialAgentRequest::new("general_translation")
            .message(crate::OfficialAgentMessage::user("hello"));
        assert_eq!(
            client.official_agent(&request).await.unwrap().id.as_deref(),
            Some("agent-1")
        );
        let mut stream = client.official_agent_stream(&request).await.unwrap();
        assert_eq!(
            stream.next().await.unwrap().unwrap().id.as_deref(),
            Some("agent-stream")
        );
        assert!(stream.next().await.is_none());

        let async_result = client
            .official_agent_async_result(&AgentAsyncResultRequest {
                async_id: "async".into(),
                agent_id: "agent".into(),
            })
            .await
            .unwrap();
        assert_eq!(async_result.status, crate::AgentAsyncStatus::Success);
        let conversation = client
            .official_agent_conversation(&AgentConversationRequest {
                agent_id: "slides_glm_agent".into(),
                conversation_id: "conversation".into(),
                custom_variables: None,
            })
            .await
            .unwrap();
        assert_eq!(conversation.conversation_id, "conversation");

        let retrieval = RetrievalAgentRequest::new(crate::RetrievalAgentConfig::new(["kb-1"]))
            .message(crate::RetrievalAgentMessage::user("question"));
        let mut stream = client
            .retrieval_agent_stream(&retrieval, Some("session-1"))
            .await
            .unwrap();
        assert_eq!(
            stream.next().await.unwrap().unwrap().kind,
            crate::RetrievalAgentEventType::SessionCreated
        );
        assert_eq!(
            stream.next().await.unwrap().unwrap().kind,
            crate::RetrievalAgentEventType::Done
        );
        assert!(stream.next().await.is_none());

        let requests = server.await.unwrap();
        assert!(requests[0].starts_with("POST /chat/completions "));
        assert!(requests[1].starts_with("POST /v1/agents "));
        assert!(requests[2].contains("\"stream\":true"));
        assert!(requests[3].starts_with("POST /v1/agents/async-result "));
        assert!(requests[4].starts_with("POST /v1/agents/conversation "));
        assert!(requests[5].starts_with("POST /zrag/agent/chat "));
        assert!(
            requests[5]
                .to_ascii_lowercase()
                .contains("x-session-id: session-1")
        );
    }

    #[tokio::test]
    async fn calls_all_zhipu_endpoint_families() {
        let mut responses = (0..31)
            .map(|_| MockResponse::json("{}"))
            .collect::<Vec<_>>();
        responses[8] = MockResponse::binary("audio");
        responses[18] = MockResponse::binary("file-data");
        let (base_url, server) = mock_server(responses).await;
        let client = ZhipuConfig::new("key")
            .base_url(&base_url)
            .http(HttpConfig {
                timeout: Duration::from_secs(5),
                ..HttpConfig::default()
            })
            .build()
            .unwrap();
        assert_eq!(client.base_url(), base_url);

        client.async_chat(&valid_chat()).await.unwrap();
        client.async_result("task id").await.unwrap();
        client
            .embedding(&EmbeddingRequest {
                model: "embedding".into(),
                input: crate::EmbeddingInput::Text("text".into()),
                dimensions: None,
                encoding_format: None,
                user_id: None,
                request_id: None,
                extra: Default::default(),
            })
            .await
            .unwrap();
        client
            .rerank(&RerankRequest {
                model: "rerank".into(),
                query: "query".into(),
                documents: vec!["document".into()],
                ..RerankRequest::default()
            })
            .await
            .unwrap();
        client
            .tokenizer(&TokenizerRequest {
                model: "tokenizer".into(),
                messages: vec![crate::ChatMessage::user("text")],
                ..TokenizerRequest::default()
            })
            .await
            .unwrap();
        let image = ImageGenerationRequest {
            model: "image".into(),
            prompt: "prompt".into(),
            ..ImageGenerationRequest::default()
        };
        client.create_image(&image).await.unwrap();
        client.create_image_async(&image).await.unwrap();
        client
            .create_video(&VideoGenerationRequest {
                model: "video".into(),
                image_url: Some(json!("https://example.com/image.png")),
                ..VideoGenerationRequest::default()
            })
            .await
            .unwrap();
        assert_eq!(
            client
                .speech(&SpeechRequest {
                    model: "speech".into(),
                    input: "text".into(),
                    voice: "voice".into(),
                    ..SpeechRequest::default()
                })
                .await
                .unwrap(),
            b"audio"
        );
        client.clone_voice(&json!({"voice":"x"})).await.unwrap();
        client.voices().await.unwrap();
        client.delete_voice(&json!({"voice":"x"})).await.unwrap();
        client.web_search(&json!({"query":"x"})).await.unwrap();
        client.read_web_page(&json!({"url":"x"})).await.unwrap();
        client.moderate(&json!({"input":"x"})).await.unwrap();
        client.parse_layout(&json!({"file":"x"})).await.unwrap();
        client
            .upload_file(FileUploadRequest {
                file_name: "input.jsonl".into(),
                file: b"data".to_vec(),
                mime_type: Some("application/jsonl".into()),
                purpose: "batch".into(),
            })
            .await
            .unwrap();
        client.files(Some("fine tune"), Some(20)).await.unwrap();
        assert_eq!(client.file_content("file/id").await.unwrap(), b"file-data");
        client.delete_file("file/id").await.unwrap();
        client.create_file_parse_task(&json!({})).await.unwrap();
        client
            .file_parse_result("task id", "text/markdown")
            .await
            .unwrap();
        client.parse_file_sync(&json!({})).await.unwrap();
        client.ocr(&json!({})).await.unwrap();
        let batch = BatchCreateRequest {
            input_file_id: "file-id".into(),
            endpoint: "/v4/chat/completions".into(),
            completion_window: crate::BatchCompletionWindow::TwentyFourHours,
            metadata: None,
        };
        client.create_batch(&batch).await.unwrap();
        client.batches(Some(10), Some("batch id")).await.unwrap();
        client.batch("batch id").await.unwrap();
        client.cancel_batch("batch id").await.unwrap();
        client.assistant(&json!({})).await.unwrap();
        client.assistants(&json!({})).await.unwrap();
        client.assistant_conversations(&json!({})).await.unwrap();

        let requests = server.await.unwrap();
        let lines = requests
            .iter()
            .map(|request| request.lines().next().unwrap())
            .collect::<Vec<_>>();
        assert_eq!(lines.len(), 31);
        assert!(lines.contains(&"GET /async-result/task%20id HTTP/1.1"));
        assert!(lines.contains(&"GET /files?purpose=fine%20tune&limit=20 HTTP/1.1"));
        assert!(lines.contains(&"GET /files/file%2Fid/content HTTP/1.1"));
        assert!(lines.contains(&"DELETE /files/file%2Fid HTTP/1.1"));
        assert!(lines.contains(&"GET /files/parser/result/task%20id/text%2Fmarkdown HTTP/1.1"));
        assert!(lines.contains(&"GET /batches?limit=10&after=batch%20id HTTP/1.1"));
        assert!(requests.iter().all(|request| {
            request
                .to_ascii_lowercase()
                .contains("authorization: bearer key")
        }));
    }

    #[tokio::test]
    async fn transcribes_and_uses_generic_requests_and_provider_trait() {
        let chat = r#"{"id":"id","choices":[{"message":{"content":"ok"}}]}"#;
        let (base_url, server) = mock_server(vec![
            MockResponse::json(r#"{"text":"words"}"#),
            MockResponse::json(r#"{"method":"post"}"#),
            MockResponse::json(r#"{"method":"get"}"#),
            MockResponse::json(chat),
            MockResponse::json(chat),
            MockResponse::json(r#"{"custom":true}"#),
        ])
        .await;
        let client = ZhipuConfig::new("key").base_url(&base_url).build().unwrap();
        let transcription = client
            .transcribe(TranscriptionRequest {
                model: "asr".into(),
                file_name: "audio.wav".into(),
                file: vec![1, 2, 3],
                mime_type: Some("audio/wav".into()),
                prompt: Some("prompt".into()),
                hotwords: vec!["Rust".into()],
                request_id: Some("request-id".into()),
                user_id: Some("user-id".into()),
            })
            .await
            .unwrap();
        assert_eq!(transcription.text, "words");
        let post: Value = client
            .request_json(Method::POST, "custom", Some(&json!({"x":1})))
            .await
            .unwrap();
        assert_eq!(post["method"], "post");
        let get: Value = client
            .request_json::<Value, Value>(Method::GET, "custom", None)
            .await
            .unwrap();
        assert_eq!(get["method"], "get");
        assert_eq!(client.name(), "zhipu");
        assert!(client.capabilities().multimodal);
        assert_eq!(
            ChatProvider::complete(&client, valid_chat())
                .await
                .unwrap()
                .text(),
            Some("ok")
        );

        let compatible = OpenAiCompatibleConfig::new("openai", "token", &base_url)
            .build()
            .unwrap();
        assert_eq!(
            compatible
                .chat_completion(&valid_chat())
                .await
                .unwrap()
                .text(),
            Some("ok")
        );
        let custom: Value = compatible
            .request_json(Method::PATCH, "models/x", Some(&json!({})))
            .await
            .unwrap();
        assert_eq!(custom["custom"], true);
        let requests = server.await.unwrap();
        assert!(
            requests[0]
                .to_ascii_lowercase()
                .contains("content-type: multipart/form-data")
        );
        assert!(!requests[4].contains("\"stream\":true"));
    }

    #[tokio::test]
    async fn endpoint_validations_fail_before_network_io() {
        let client = ZhipuConfig::new("key")
            .base_url("http://127.0.0.1:1")
            .build()
            .unwrap();
        assert_validation(client.async_result(" ").await, "task id");
        assert_validation(
            client
                .embedding(&EmbeddingRequest {
                    model: " ".into(),
                    input: crate::EmbeddingInput::Text("x".into()),
                    dimensions: None,
                    encoding_format: None,
                    user_id: None,
                    request_id: None,
                    extra: Default::default(),
                })
                .await,
            "embedding model",
        );
        assert_validation(client.rerank(&RerankRequest::default()).await, "rerank");
        assert_validation(
            client.tokenizer(&TokenizerRequest::default()).await,
            "tokenizer",
        );
        assert_validation(
            client
                .create_image(&ImageGenerationRequest::default())
                .await,
            "image generation",
        );
        assert_validation(
            client
                .create_image_async(&ImageGenerationRequest::default())
                .await,
            "image generation",
        );
        assert_validation(
            client
                .create_video(&VideoGenerationRequest::default())
                .await,
            "video model",
        );
        let mut video = VideoGenerationRequest {
            model: "video".into(),
            ..VideoGenerationRequest::default()
        };
        assert_validation(client.create_video(&video).await, "prompt or image_url");
        video.prompt = Some("prompt".into());
        assert!(client.create_video(&video).await.is_err());
        assert_validation(
            client
                .transcribe(TranscriptionRequest {
                    model: String::new(),
                    file_name: String::new(),
                    file: Vec::new(),
                    mime_type: None,
                    prompt: None,
                    hotwords: Vec::new(),
                    request_id: None,
                    user_id: None,
                })
                .await,
            "transcription",
        );
        assert_validation(client.speech(&SpeechRequest::default()).await, "speech");
        assert_validation(
            client
                .upload_file(FileUploadRequest {
                    file_name: String::new(),
                    file: Vec::new(),
                    mime_type: None,
                    purpose: String::new(),
                })
                .await,
            "file upload",
        );
        assert_validation(client.delete_file("").await, "file id");
        assert_validation(client.file_content("").await, "file id");
        assert_validation(client.file_parse_result("", "text").await, "parser task id");
        assert_validation(
            client.file_parse_result("task", "").await,
            "parser format type",
        );
        assert!(matches!(
            client.create_batch(&BatchCreateRequest::default()).await,
            Err(SdkError::Batch(BatchError::MissingCreateFields))
        ));
        assert_validation(client.batch("").await, "batch id");
        assert_validation(client.cancel_batch("").await, "batch id");

        let invalid_mime = TranscriptionRequest {
            model: "asr".into(),
            file_name: "a".into(),
            file: vec![1],
            mime_type: Some("bad\nvalue".into()),
            prompt: None,
            hotwords: Vec::new(),
            request_id: None,
            user_id: None,
        };
        assert_validation(client.transcribe(invalid_mime).await, "builder error");
        assert_validation(
            client
                .upload_file(FileUploadRequest {
                    file_name: "a".into(),
                    file: vec![1],
                    mime_type: Some("bad\nvalue".into()),
                    purpose: "batch".into(),
                })
                .await,
            "builder error",
        );
    }

    #[test]
    fn client_configuration_rejects_invalid_values() {
        assert!(ZhipuClient::new("").is_err());
        assert!(
            OpenAiCompatibleConfig::new("", "key", "https://example.com")
                .build()
                .is_err()
        );
        assert!(
            OpenAiCompatibleConfig::new("name", "key", "https://example.com")
                .chat_path(" ")
                .build()
                .is_err()
        );
        assert!(
            ZhipuConfig::new("key")
                .authentication(ZhipuAuthentication::bearer("explicit"))
                .base_url("invalid")
                .build()
                .is_err()
        );
    }
}
