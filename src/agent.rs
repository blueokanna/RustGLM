use std::collections::BTreeMap;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use async_stream::try_stream;
use async_trait::async_trait;
use futures_util::Stream;
use nextjson::{Map, Value};
use nextjson::{NsonDeserialize as Deserialize, NsonSerialize as Serialize};

use crate::wire_enum;

use crate::client::{OpenAiCompatibleConfig, ZHIPU_BASE_URL, ZhipuConfig};
use crate::{
    ChatCompletionRequest, ChatCompletionResponse, ChatMessage, ChatProvider, ConversationMemory,
    ExtraFields, FunctionDefinition, MemoryDocument, MessageContent, Result, SdkError, Tool, Usage,
};

pub type OfficialAgentStream = Pin<Box<dyn Stream<Item = Result<OfficialAgentResponse>> + Send>>;
pub type RetrievalAgentStream = Pin<Box<dyn Stream<Item = Result<RetrievalAgentEvent>> + Send>>;

/// Monotonic counter that keeps agent memory ids unique even within the same nanosecond.
static AGENT_MEMORY_SEQUENCE: AtomicU64 = AtomicU64::new(1);

wire_enum! {
    /// Official agent message role.
    pub enum OfficialAgentRole {
        System => "system",
        User => "user",
        Assistant => "assistant",
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum OfficialAgentInputPart {
    Text { text: String },
    FileId { file_id: String },
    FileUrl { file_url: String },
    ImageUrl { image_url: String },
}

impl OfficialAgentInputPart {
    pub fn text(value: impl Into<String>) -> Self {
        Self::Text { text: value.into() }
    }

    pub fn file_id(value: impl Into<String>) -> Self {
        Self::FileId {
            file_id: value.into(),
        }
    }

    pub fn file_url(value: impl Into<String>) -> Self {
        Self::FileUrl {
            file_url: value.into(),
        }
    }

    pub fn image_url(value: impl Into<String>) -> Self {
        Self::ImageUrl {
            image_url: value.into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum OfficialAgentMessageContent {
    Text(String),
    Part(OfficialAgentInputPart),
    Parts(Vec<OfficialAgentInputPart>),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OfficialAgentMessage {
    pub role: OfficialAgentRole,
    pub content: OfficialAgentMessageContent,
}

impl OfficialAgentMessage {
    pub fn user(value: impl Into<String>) -> Self {
        Self {
            role: OfficialAgentRole::User,
            content: OfficialAgentMessageContent::Text(value.into()),
        }
    }

    pub fn multimodal(parts: Vec<OfficialAgentInputPart>) -> Self {
        Self {
            role: OfficialAgentRole::User,
            content: OfficialAgentMessageContent::Parts(parts),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OfficialAgentRequest {
    pub agent_id: String,
    #[serde(default, skip_serializing_if = "is_false")]
    pub stream: bool,
    pub messages: Vec<OfficialAgentMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_variables: Option<Value>,
    #[serde(flatten, default)]
    pub extra: ExtraFields,
}

impl OfficialAgentRequest {
    pub fn new(agent_id: impl Into<String>) -> Self {
        Self {
            agent_id: agent_id.into(),
            stream: false,
            messages: Vec::new(),
            custom_variables: None,
            extra: Map::new(),
        }
    }

    pub fn message(mut self, message: OfficialAgentMessage) -> Self {
        self.messages.push(message);
        self
    }

    pub fn custom_variables(mut self, value: impl Serialize) -> nextjson::Result<Self> {
        self.custom_variables = Some(nextjson::to_value(&value)?);
        Ok(self)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TranslationAgentVariables {
    #[serde(default = "default_source_language")]
    pub source_lang: String,
    #[serde(default = "default_target_language")]
    pub target_lang: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub glossary: Option<String>,
    #[serde(default = "default_translation_strategy")]
    pub strategy: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strategy_config: Option<Value>,
}

impl Default for TranslationAgentVariables {
    fn default() -> Self {
        Self {
            source_lang: default_source_language(),
            target_lang: default_target_language(),
            glossary: None,
            strategy: default_translation_strategy(),
            strategy_config: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum OfficialAgentOutputPart {
    Text { text: String },
    FileUrl { file_url: String },
    ImageUrl { image_url: String },
    AudioUrl { audio_url: String },
    VideoUrl { video_url: String },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum OfficialAgentOutputContent {
    Text(String),
    Part(OfficialAgentOutputPart),
    Parts(Vec<OfficialAgentOutputPart>),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OfficialAgentOutputMessage {
    pub role: OfficialAgentRole,
    pub content: OfficialAgentOutputContent,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct OfficialAgentChoice {
    #[serde(default)]
    pub index: u32,
    #[serde(default)]
    pub messages: Vec<OfficialAgentOutputMessage>,
    #[serde(default)]
    pub finish_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct OfficialAgentResponse {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub agent_id: Option<String>,
    #[serde(default)]
    pub conversation_id: Option<String>,
    #[serde(default)]
    pub async_id: Option<String>,
    #[serde(default)]
    pub choices: Vec<OfficialAgentChoice>,
    #[serde(default)]
    pub usage: Option<Usage>,
    #[serde(flatten, default)]
    pub extra: ExtraFields,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AgentAsyncResultRequest {
    pub async_id: String,
    pub agent_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct AgentAsyncResultResponse {
    #[serde(default)]
    pub agent_id: String,
    #[serde(default)]
    pub async_id: String,
    #[serde(default)]
    pub status: AgentAsyncStatus,
    #[serde(default)]
    pub choices: Vec<OfficialAgentChoice>,
    #[serde(default)]
    pub usage: Option<Usage>,
    #[serde(flatten, default)]
    pub extra: ExtraFields,
}

wire_enum! {
    /// Async agent task status.
    pub enum AgentAsyncStatus {
        Success => "success",
        Failed => "failed",
        Pending => "pending",
    }
}

#[allow(clippy::derivable_impls)]
impl Default for AgentAsyncStatus {
    fn default() -> Self {
        Self::Pending
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct AgentConversationRequest {
    pub agent_id: String,
    pub conversation_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_variables: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct AgentConversationResponse {
    #[serde(default)]
    pub conversation_id: String,
    #[serde(default)]
    pub agent_id: String,
    #[serde(default)]
    pub choices: Vec<AgentConversationChoice>,
    #[serde(default)]
    pub error: Option<AgentConversationError>,
    #[serde(flatten, default)]
    pub extra: ExtraFields,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct AgentConversationChoice {
    #[serde(default)]
    pub message: Vec<AgentConversationMessage>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct AgentConversationMessage {
    #[serde(default)]
    pub role: String,
    #[serde(default)]
    pub content: Vec<AgentConversationContent>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct AgentConversationContent {
    #[serde(rename = "type", default)]
    pub kind: String,
    #[serde(default)]
    pub tag_cn: Option<String>,
    #[serde(default)]
    pub tag_en: Option<String>,
    #[serde(default)]
    pub file_url: Option<String>,
    #[serde(default)]
    pub image_url: Option<String>,
    #[serde(flatten, default)]
    pub extra: ExtraFields,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct AgentConversationError {
    #[serde(default)]
    pub code: String,
    #[serde(default)]
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum RetrievalAgentMessageContent {
    Text(String),
    Parts(Vec<RetrievalAgentContentPart>),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RetrievalAgentContentPart {
    Text { text: String },
    ImageUrl { image_url: RetrievalAgentImageUrl },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RetrievalAgentImageUrl {
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RetrievalAgentMessage {
    pub role: String,
    pub content: RetrievalAgentMessageContent,
}

impl RetrievalAgentMessage {
    pub fn user(value: impl Into<String>) -> Self {
        Self {
            role: "user".into(),
            content: RetrievalAgentMessageContent::Text(value.into()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RetrievalAgentConfig {
    pub know_ids: Vec<String>,
    #[serde(default = "default_top_k")]
    pub top_k: u32,
    #[serde(default = "default_top_n")]
    pub top_n: u32,
    #[serde(default)]
    pub enable_rerank: bool,
    #[serde(default = "default_similarity_threshold")]
    pub similarity_threshold: f32,
}

impl RetrievalAgentConfig {
    pub fn new(know_ids: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self {
            know_ids: know_ids.into_iter().map(Into::into).collect(),
            top_k: default_top_k(),
            top_n: default_top_n(),
            enable_rerank: false,
            similarity_threshold: default_similarity_threshold(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RetrievalAgentRequest {
    pub messages: Vec<RetrievalAgentMessage>,
    #[serde(default = "default_retrieval_model")]
    pub model: String,
    #[serde(default = "default_agent_temperature")]
    pub temperature: f32,
    #[serde(default = "default_agent_steps")]
    pub max_steps: u32,
    pub retrieval: RetrievalAgentConfig,
    #[serde(default)]
    pub enable_thinking: bool,
}

impl RetrievalAgentRequest {
    pub fn new(retrieval: RetrievalAgentConfig) -> Self {
        Self {
            messages: Vec::new(),
            model: default_retrieval_model(),
            temperature: default_agent_temperature(),
            max_steps: default_agent_steps(),
            retrieval,
            enable_thinking: false,
        }
    }

    pub fn message(mut self, value: RetrievalAgentMessage) -> Self {
        self.messages.push(value);
        self
    }
}

wire_enum! {
    /// Retrieval agent event type.
    pub enum RetrievalAgentEventType {
        SessionCreated => "session_created",
        Reasoning => "reasoning",
        Thought => "thought",
        ToolCall => "tool_call",
        ToolResult => "tool_result",
        Answer => "answer",
        Done => "done",
        Error => "error",
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct RetrievalAgentUsage {
    #[serde(default)]
    pub prompt_tokens: u64,
    #[serde(default)]
    pub completion_tokens: u64,
    #[serde(default)]
    pub total_tokens: u64,
    #[serde(default)]
    pub total_calls: u64,
    #[serde(default)]
    pub prompt_tokens_details: Option<Value>,
    #[serde(default)]
    pub completion_tokens_details: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RetrievalAgentEvent {
    #[serde(rename = "type")]
    pub kind: RetrievalAgentEventType,
    #[serde(default, rename = "sessionId")]
    pub session_id: Option<String>,
    #[serde(default, rename = "messageId")]
    pub message_id: Option<String>,
    #[serde(default)]
    pub data: Option<Value>,
    #[serde(default)]
    pub usage: Option<RetrievalAgentUsage>,
}

impl RetrievalAgentEvent {
    pub fn text(&self) -> Option<&str> {
        matches!(
            self.kind,
            RetrievalAgentEventType::Reasoning
                | RetrievalAgentEventType::Thought
                | RetrievalAgentEventType::Answer
        )
        .then(|| self.data.as_ref()?.as_str())
        .flatten()
    }

    pub fn tool_call(&self) -> Result<Option<RetrievalAgentToolCall>> {
        self.parse_data(RetrievalAgentEventType::ToolCall)
    }

    pub fn tool_result(&self) -> Result<Option<RetrievalAgentToolResult>> {
        self.parse_data(RetrievalAgentEventType::ToolResult)
    }

    pub fn error(&self) -> Result<Option<RetrievalAgentError>> {
        self.parse_data(RetrievalAgentEventType::Error)
    }

    fn parse_data<T: for<'de> Deserialize<'de>>(
        &self,
        expected: RetrievalAgentEventType,
    ) -> Result<Option<T>> {
        if self.kind != expected {
            return Ok(None);
        }
        let data = self.data.clone().ok_or_else(|| SdkError::Decode {
            message: "retrieval agent event is missing data".into(),
            body: String::new(),
        })?;
        nextjson::from_value(data.clone())
            .map(Some)
            .map_err(|error| SdkError::Decode {
                message: error.to_string(),
                body: data.to_string(),
            })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RetrievalAgentToolCall {
    #[serde(rename = "callId")]
    pub call_id: String,
    #[serde(rename = "toolName")]
    pub tool_name: String,
    #[serde(default)]
    pub arguments: Map,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RetrievalAgentToolResult {
    #[serde(rename = "callId")]
    pub call_id: String,
    #[serde(rename = "toolName")]
    pub tool_name: String,
    #[serde(default = "null_value")]
    pub result: Value,
    pub status: RetrievalAgentToolStatus,
    #[serde(default, rename = "durationMs")]
    pub duration_ms: u64,
}

wire_enum! {
    /// Retrieval agent tool result status.
    pub enum RetrievalAgentToolStatus {
        Success => "success",
        Error => "error",
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RetrievalAgentError {
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct AgentPersona {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub role: String,
    #[serde(default)]
    pub background: String,
    #[serde(default)]
    pub traits: Vec<String>,
    #[serde(default)]
    pub speaking_style: String,
    #[serde(default)]
    pub language: Option<String>,
    #[serde(default)]
    pub instructions: Vec<String>,
    #[serde(default)]
    pub boundaries: Vec<String>,
}

impl AgentPersona {
    pub fn new(name: impl Into<String>, role: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            role: role.into(),
            ..Self::default()
        }
    }

    pub fn background(mut self, value: impl Into<String>) -> Self {
        self.background = value.into();
        self
    }

    pub fn trait_value(mut self, value: impl Into<String>) -> Self {
        self.traits.push(value.into());
        self
    }

    pub fn speaking_style(mut self, value: impl Into<String>) -> Self {
        self.speaking_style = value.into();
        self
    }

    pub fn language(mut self, value: impl Into<String>) -> Self {
        self.language = Some(value.into());
        self
    }

    pub fn instruction(mut self, value: impl Into<String>) -> Self {
        self.instructions.push(value.into());
        self
    }

    pub fn boundary(mut self, value: impl Into<String>) -> Self {
        self.boundaries.push(value.into());
        self
    }

    pub fn validate(&self) -> Result<()> {
        if self.name.trim().is_empty() || self.role.trim().is_empty() {
            return Err(SdkError::Configuration(
                "agent persona requires a name and role".into(),
            ));
        }
        Ok(())
    }

    pub fn system_prompt(&self) -> Result<String> {
        self.validate()?;
        let mut sections = vec![
            format!("Identity: {}", self.name.trim()),
            format!("Role: {}", self.role.trim()),
        ];
        push_prompt(&mut sections, "Background", &self.background);
        if !self.traits.is_empty() {
            sections.push(format!("Traits: {}", self.traits.join("; ")));
        }
        push_prompt(&mut sections, "Speaking style", &self.speaking_style);
        if let Some(language) = self
            .language
            .as_deref()
            .filter(|value| !value.trim().is_empty())
        {
            sections.push(format!("Language: {}", language.trim()));
        }
        if !self.instructions.is_empty() {
            sections.push(format!("Instructions: {}", self.instructions.join("; ")));
        }
        if !self.boundaries.is_empty() {
            sections.push(format!("Boundaries: {}", self.boundaries.join("; ")));
        }
        Ok(sections.join("\n"))
    }
}

wire_enum! {
    /// Agent provider protocol.
    pub enum AgentProtocol {
        Zhipu => "zhipu",
        OpenAiCompatible => "openai_compatible",
        ; strict
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CredentialReference {
    pub id: String,
}

impl CredentialReference {
    pub fn new(id: impl Into<String>) -> Self {
        Self { id: id.into() }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AgentProviderDeployment {
    pub protocol: AgentProtocol,
    pub name: String,
    pub base_url: String,
    #[serde(default = "default_chat_path")]
    pub chat_path: String,
    pub credential: CredentialReference,
}

impl AgentProviderDeployment {
    pub fn zhipu() -> Self {
        Self {
            protocol: AgentProtocol::Zhipu,
            name: "zhipu".into(),
            base_url: ZHIPU_BASE_URL.into(),
            chat_path: default_chat_path(),
            credential: CredentialReference::new("ZHIPU_API_KEY"),
        }
    }

    pub fn openai_compatible(
        name: impl Into<String>,
        base_url: impl Into<String>,
        credential_id: impl Into<String>,
    ) -> Self {
        Self {
            protocol: AgentProtocol::OpenAiCompatible,
            name: name.into(),
            base_url: base_url.into(),
            chat_path: default_chat_path(),
            credential: CredentialReference::new(credential_id),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum AgentHistoryPolicy {
    #[default]
    Stateless,
    Recent {
        max_messages: usize,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AgentManifest {
    #[serde(default = "default_manifest_version")]
    pub version: u32,
    pub provider: AgentProviderDeployment,
    pub model: String,
    pub persona: AgentPersona,
    #[serde(default = "default_agent_temperature")]
    pub temperature: f32,
    #[serde(default)]
    pub max_tokens: Option<u32>,
    #[serde(default = "default_agent_steps")]
    pub max_steps: u32,
    #[serde(default)]
    pub history: AgentHistoryPolicy,
}

impl AgentManifest {
    pub fn new(model: impl Into<String>, persona: AgentPersona) -> Self {
        Self {
            version: default_manifest_version(),
            provider: AgentProviderDeployment::zhipu(),
            model: model.into(),
            persona,
            temperature: default_agent_temperature(),
            max_tokens: None,
            max_steps: default_agent_steps(),
            history: AgentHistoryPolicy::Stateless,
        }
    }

    pub fn provider(mut self, value: AgentProviderDeployment) -> Self {
        self.provider = value;
        self
    }

    pub fn history(mut self, value: AgentHistoryPolicy) -> Self {
        self.history = value;
        self
    }

    pub fn validate(&self) -> Result<()> {
        self.persona.validate()?;
        if self.version != 1 {
            return Err(SdkError::Configuration(
                format!("unsupported agent manifest version {}", self.version).into(),
            ));
        }
        if self.model.trim().is_empty()
            || self.provider.name.trim().is_empty()
            || self.provider.base_url.trim().is_empty()
            || self.provider.credential.id.trim().is_empty()
        {
            return Err(SdkError::Configuration(
                "agent manifest provider, model, and credential reference cannot be empty".into(),
            ));
        }
        let temperature_max = match self.provider.protocol {
            AgentProtocol::Zhipu => 1.0,
            AgentProtocol::OpenAiCompatible => 2.0,
        };
        if !(0.0..=temperature_max).contains(&self.temperature) || self.max_steps == 0 {
            return Err(SdkError::Configuration(
                "agent temperature or maximum steps are invalid".into(),
            ));
        }
        if matches!(self.history, AgentHistoryPolicy::Recent { max_messages: 0 }) {
            return Err(SdkError::Configuration(
                "agent history limit must be greater than zero".into(),
            ));
        }
        Ok(())
    }

    pub fn to_json(&self) -> Result<String> {
        self.validate()?;
        nextjson::to_string_pretty(self)
            .map_err(|error| SdkError::Configuration(error.to_string().into()))
    }

    pub fn from_json(value: &str) -> Result<Self> {
        let manifest: Self = nextjson::from_str(value)
            .map_err(|error| SdkError::Configuration(error.to_string().into()))?;
        manifest.validate()?;
        Ok(manifest)
    }

    pub fn build_provider(&self, resolver: &dyn SecretResolver) -> Result<Arc<dyn ChatProvider>> {
        self.validate()?;
        let credential = resolver.resolve(&self.provider.credential)?;
        match self.provider.protocol {
            AgentProtocol::Zhipu => Ok(Arc::new(
                ZhipuConfig::new(credential)
                    .base_url(&self.provider.base_url)
                    .build()?,
            )),
            AgentProtocol::OpenAiCompatible => Ok(Arc::new(
                OpenAiCompatibleConfig::new(
                    &self.provider.name,
                    credential,
                    &self.provider.base_url,
                )
                .chat_path(&self.provider.chat_path)
                .build()?,
            )),
        }
    }
}

pub trait SecretResolver: Send + Sync {
    fn resolve(&self, reference: &CredentialReference) -> Result<String>;
}

#[derive(Debug, Clone, Copy, Default)]
pub struct EnvironmentSecretResolver;

impl SecretResolver for EnvironmentSecretResolver {
    fn resolve(&self, reference: &CredentialReference) -> Result<String> {
        std::env::var(&reference.id).map_err(|_| {
            SdkError::Configuration(
                format!(
                    "credential reference {} could not be resolved",
                    reference.id
                )
                .into(),
            )
        })
    }
}

#[async_trait]
pub trait AgentTool: Send + Sync {
    fn definition(&self) -> FunctionDefinition;
    async fn execute(&self, arguments: Value) -> Result<Value>;
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AgentToolExecution {
    pub call_id: String,
    pub name: String,
    pub arguments: Value,
    pub output: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AgentRunResult {
    pub response: ChatCompletionResponse,
    pub model_steps: u32,
    pub tool_executions: Vec<AgentToolExecution>,
}

pub struct AgentRuntime {
    provider: Arc<dyn ChatProvider>,
    manifest: AgentManifest,
    tools: BTreeMap<String, Arc<dyn AgentTool>>,
    history: Vec<ChatMessage>,
    memory: Option<Arc<dyn ConversationMemory>>,
    recall_limit: usize,
}

impl AgentRuntime {
    pub fn new(provider: Arc<dyn ChatProvider>, manifest: AgentManifest) -> Result<Self> {
        manifest.validate()?;
        Ok(Self {
            provider,
            manifest,
            tools: BTreeMap::new(),
            history: Vec::new(),
            memory: None,
            recall_limit: 4,
        })
    }

    pub fn from_manifest(manifest: AgentManifest, resolver: &dyn SecretResolver) -> Result<Self> {
        let provider = manifest.build_provider(resolver)?;
        Self::new(provider, manifest)
    }

    pub fn manifest(&self) -> &AgentManifest {
        &self.manifest
    }

    pub fn history(&self) -> &[ChatMessage] {
        &self.history
    }

    pub fn clear_history(&mut self) {
        self.history.clear();
    }

    pub async fn clear_memory(&self) -> Result<()> {
        if let Some(memory) = &self.memory {
            memory.clear().await?;
        }
        Ok(())
    }

    pub fn semantic_memory(
        mut self,
        memory: Arc<dyn ConversationMemory>,
        recall_limit: usize,
    ) -> Result<Self> {
        if recall_limit == 0 {
            return Err(SdkError::Configuration(
                "agent semantic recall limit must be greater than zero".into(),
            ));
        }
        self.memory = Some(memory);
        self.recall_limit = recall_limit;
        Ok(self)
    }

    pub fn register_tool<T>(&mut self, tool: T) -> Result<()>
    where
        T: AgentTool + 'static,
    {
        self.register_shared_tool(Arc::new(tool))
    }

    pub fn register_shared_tool(&mut self, tool: Arc<dyn AgentTool>) -> Result<()> {
        let definition = tool.definition();
        let name = definition.name.trim();
        if name.is_empty() {
            return Err(SdkError::Configuration(
                "agent tool name cannot be empty".into(),
            ));
        }
        if self.tools.contains_key(name) {
            return Err(SdkError::Configuration(
                format!("agent tool {name} is already registered").into(),
            ));
        }
        self.tools.insert(name.to_owned(), tool);
        Ok(())
    }

    pub async fn run(&mut self, input: impl Into<String>) -> Result<AgentRunResult> {
        let input = input.into();
        if input.trim().is_empty() {
            return Err(SdkError::Validation("agent input cannot be empty".into()));
        }
        let mut messages = vec![ChatMessage::system(self.manifest.persona.system_prompt()?)];
        if let Some(memory) = &self.memory {
            let recalled = memory.recall(&input, self.recall_limit).await?;
            if !recalled.is_empty() {
                let values = recalled
                    .into_iter()
                    .map(|item| item.document.text)
                    .collect::<Vec<_>>();
                messages.push(ChatMessage::system(format!(
                    "Relevant prior context:\n{}",
                    nextjson::to_string(&values)
                        .map_err(|error| SdkError::Validation(error.to_string().into()))?
                )));
            }
        }
        messages.extend(self.history.iter().cloned());
        messages.push(ChatMessage::user(&input));
        let definitions = self
            .tools
            .values()
            .map(|tool| Tool::function(tool.definition()))
            .collect::<Vec<_>>();
        let mut executions = Vec::new();
        for step in 1..=self.manifest.max_steps {
            let mut request = ChatCompletionRequest::new(&self.manifest.model)
                .messages(messages.iter().cloned())
                .temperature(self.manifest.temperature);
            request.max_tokens = self.manifest.max_tokens;
            if !definitions.is_empty() {
                request.tools = Some(definitions.clone());
            }
            let response = self.provider.complete(request).await?;
            let message = response
                .choices
                .first()
                .map(|choice| &choice.message)
                .ok_or_else(|| SdkError::Agent("model response contained no choices".into()))?;
            if message.tool_calls.is_empty() {
                self.commit_turn(&input, response.text()).await?;
                return Ok(AgentRunResult {
                    response,
                    model_steps: step,
                    tool_executions: executions,
                });
            }
            let calls = message.tool_calls.clone();
            messages.push(ChatMessage {
                role: crate::MessageRole::Assistant,
                content: message.content.as_ref().and_then(|content| match content {
                    crate::ResponseContent::Text(value) => {
                        Some(MessageContent::Text(value.to_owned()))
                    }
                    crate::ResponseContent::Parts(_) => None,
                }),
                name: None,
                tool_call_id: None,
                tool_calls: Some(calls.clone()),
                reasoning_content: message.reasoning_content.clone(),
                extra: message.extra.clone(),
            });
            for call in calls {
                let function = call.function.ok_or_else(|| {
                    SdkError::Tool(format!("tool call {} has no function payload", call.id).into())
                })?;
                let arguments =
                    nextjson::from_str::<Value>(&function.arguments).map_err(|error| {
                        SdkError::Tool(
                            format!(
                                "tool {} arguments are not valid JSON: {error}",
                                function.name
                            )
                            .into(),
                        )
                    })?;
                let tool = self.tools.get(&function.name).ok_or_else(|| {
                    SdkError::Tool(format!("tool {} is not registered", function.name).into())
                })?;
                let output = tool.execute(arguments.clone()).await?;
                let output_text = nextjson::to_string(&output)
                    .map_err(|error| SdkError::Tool(error.to_string().into()))?;
                messages.push(ChatMessage::tool_result(&call.id, output_text));
                executions.push(AgentToolExecution {
                    call_id: call.id,
                    name: function.name,
                    arguments,
                    output,
                });
            }
        }
        Err(SdkError::Agent(
            format!(
                "agent exceeded the configured {} model steps",
                self.manifest.max_steps
            )
            .into(),
        ))
    }

    async fn commit_turn(&mut self, input: &str, output: Option<&str>) -> Result<()> {
        let Some(output) = output else {
            return Ok(());
        };
        if let AgentHistoryPolicy::Recent { max_messages } = self.manifest.history {
            self.history.push(ChatMessage::user(input));
            self.history.push(ChatMessage::assistant(output));
            let overflow = self.history.len().saturating_sub(max_messages);
            if overflow > 0 {
                self.history.drain(..overflow);
            }
        }
        if let Some(memory) = &self.memory {
            memory
                .remember(
                    MemoryDocument::new(
                        agent_memory_id()?,
                        format!("User: {input}\nAssistant: {output}"),
                    )
                    .metadata("source", "agent"),
                )
                .await?;
        }
        Ok(())
    }
}

#[cfg(feature = "agents")]
pub(crate) fn official_agent_stream(response: reqwest::Response) -> OfficialAgentStream {
    decode_sse_stream(response)
}

#[cfg(feature = "rag")]
pub(crate) fn retrieval_agent_stream(response: reqwest::Response) -> RetrievalAgentStream {
    decode_sse_stream(response)
}

fn decode_sse_stream<T>(
    response: reqwest::Response,
) -> Pin<Box<dyn Stream<Item = Result<T>> + Send>>
where
    T: for<'de> Deserialize<'de> + Send + 'static,
{
    let stream = try_stream! {
        let mut response = response;
        let mut decoder = crate::sse::SseDecoder::<T>::default();
        while let Some(bytes) = response.chunk().await? {
            for value in decoder.push(&bytes)? {
                yield value;
            }
        }
        for value in decoder.finish()? {
            yield value;
        }
    };
    Box::pin(stream)
}

/// Default JSON null used by optional `Value` fields that nextjson cannot default-construct.
fn null_value() -> Value {
    Value::Null
}

fn push_prompt(sections: &mut Vec<String>, label: &str, value: &str) {
    if !value.trim().is_empty() {
        sections.push(format!("{label}: {}", value.trim()));
    }
}

fn agent_memory_id() -> Result<String> {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| SdkError::Configuration("system clock is before Unix epoch".into()))?
        .as_nanos();
    let sequence = AGENT_MEMORY_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    Ok(format!("agent-{timestamp}-{sequence}"))
}

fn default_source_language() -> String {
    "auto".into()
}

fn default_target_language() -> String {
    "zh-CN".into()
}

fn default_translation_strategy() -> String {
    "general".into()
}

fn default_retrieval_model() -> String {
    "glm-5v-turbo".into()
}

fn default_chat_path() -> String {
    "chat/completions".into()
}

fn default_manifest_version() -> u32 {
    1
}

fn default_agent_temperature() -> f32 {
    0.7
}

fn default_agent_steps() -> u32 {
    10
}

fn default_top_k() -> u32 {
    8
}

fn default_top_n() -> u32 {
    10
}

fn default_similarity_threshold() -> f32 {
    0.2
}

fn is_false(value: &bool) -> bool {
    !*value
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;
    use std::sync::Mutex;
    use std::sync::atomic::{AtomicBool, Ordering};

    use futures_util::stream;
    use nextjson::json;

    use super::*;
    use crate::{
        ChatChoice, ChatResponseMessage, FunctionCall, MessageRole, ProviderCapabilities,
        ResponseContent, ResponseContentPart, ToolCall,
    };

    struct StaticResolver(String);

    impl SecretResolver for StaticResolver {
        fn resolve(&self, _: &CredentialReference) -> Result<String> {
            Ok(self.0.clone())
        }
    }

    struct MockProvider {
        responses: Mutex<VecDeque<ChatCompletionResponse>>,
        requests: Mutex<Vec<ChatCompletionRequest>>,
    }

    #[async_trait]
    impl ChatProvider for MockProvider {
        fn name(&self) -> &str {
            "mock"
        }

        fn capabilities(&self) -> ProviderCapabilities {
            ProviderCapabilities::openai_compatible()
        }

        async fn complete(&self, request: ChatCompletionRequest) -> Result<ChatCompletionResponse> {
            self.requests.lock().unwrap().push(request);
            self.responses
                .lock()
                .unwrap()
                .pop_front()
                .ok_or_else(|| SdkError::Agent("missing mock response".into()))
        }

        async fn stream(&self, _: ChatCompletionRequest) -> Result<crate::ChatStream> {
            Ok(Box::pin(stream::empty()))
        }
    }

    struct EchoTool;

    struct EmptyNameTool;

    #[async_trait]
    impl AgentTool for EmptyNameTool {
        fn definition(&self) -> FunctionDefinition {
            FunctionDefinition::new("", json!({"type":"object"}))
        }

        async fn execute(&self, _: Value) -> Result<Value> {
            Ok(Value::Null)
        }
    }

    #[derive(Default)]
    struct RecordingMemory {
        documents: Mutex<Vec<MemoryDocument>>,
        cleared: AtomicBool,
    }

    #[async_trait]
    impl ConversationMemory for RecordingMemory {
        async fn remember(&self, document: MemoryDocument) -> Result<()> {
            self.documents.lock().unwrap().push(document);
            Ok(())
        }

        async fn recall(&self, _: &str, _: usize) -> Result<Vec<crate::MemoryMatch>> {
            Ok(vec![crate::MemoryMatch {
                document: MemoryDocument::new("prior", "remembered context"),
                score: 1.0,
            }])
        }

        async fn clear(&self) -> Result<()> {
            self.cleared.store(true, Ordering::SeqCst);
            Ok(())
        }
    }

    #[async_trait]
    impl AgentTool for EchoTool {
        fn definition(&self) -> FunctionDefinition {
            FunctionDefinition::new(
                "echo",
                json!({"type":"object","properties":{"value":{"type":"string"}}}),
            )
        }

        async fn execute(&self, arguments: Value) -> Result<Value> {
            Ok(json!({"echoed":arguments["value"]}))
        }
    }

    fn response(text: Option<&str>, tool_calls: Vec<ToolCall>) -> ChatCompletionResponse {
        ChatCompletionResponse {
            choices: vec![ChatChoice {
                index: 0,
                message: ChatResponseMessage {
                    role: Some(MessageRole::Assistant),
                    content: text.map(|value| ResponseContent::Text(value.into())),
                    tool_calls,
                    ..Default::default()
                },
                finish_reason: Some("stop".into()),
            }],
            ..Default::default()
        }
    }

    fn tool_call() -> ToolCall {
        ToolCall {
            id: "call-1".into(),
            kind: "function".into(),
            function: Some(FunctionCall {
                name: "echo".into(),
                arguments: "{\"value\":\"hello\"}".into(),
            }),
            extra: Map::new(),
        }
    }

    fn manifest() -> AgentManifest {
        AgentManifest::new(
            "glm-5.3",
            AgentPersona::new("Lin", "technical companion")
                .background("Rust engineer")
                .trait_value("precise")
                .speaking_style("concise")
                .language("English")
                .instruction("Use tools when useful")
                .boundary("Do not invent tool results"),
        )
        .history(AgentHistoryPolicy::Recent { max_messages: 4 })
    }

    #[test]
    fn official_agent_request_serializes_multimodal_content() {
        let request = OfficialAgentRequest::new("general_translation")
            .message(OfficialAgentMessage::user("hello"))
            .message(OfficialAgentMessage::multimodal(vec![
                OfficialAgentInputPart::text("translate"),
                OfficialAgentInputPart::file_id("file-1"),
                OfficialAgentInputPart::file_url("https://example.com/a.pdf"),
                OfficialAgentInputPart::image_url("https://example.com/a.png"),
            ]))
            .custom_variables(TranslationAgentVariables::default())
            .unwrap();
        let value = nextjson::to_value(&request).unwrap();
        assert_eq!(value["agent_id"].as_str(), Some("general_translation"));
        assert_eq!(
            value["messages"][1]["content"][1]["type"].as_str(),
            Some("file_id")
        );
        assert_eq!(
            value["custom_variables"]["strategy"].as_str(),
            Some("general")
        );
        assert!(value.get("stream").is_none());
    }

    #[test]
    fn official_agent_output_supports_every_documented_modality() {
        let response: OfficialAgentResponse = nextjson::from_value(json!({
            "id":"agent-1",
            "choices":[{"index":0,"messages":[
                {"role":"assistant","content":"plain"},
                {"role":"assistant","content":{"type":"audio_url","audio_url":"https://example.com/a.wav"}},
                {"role":"assistant","content":[
                    {"type":"text","text":"text"},
                    {"type":"file_url","file_url":"https://example.com/a.pdf"},
                    {"type":"image_url","image_url":"https://example.com/a.png"},
                    {"type":"video_url","video_url":"https://example.com/a.mp4"}
                ]}
            ]}]
        }))
        .unwrap();
        assert_eq!(response.choices[0].messages.len(), 3);
    }

    #[test]
    fn translation_defaults_match_official_schema() {
        let value = nextjson::to_value(&TranslationAgentVariables::default()).unwrap();
        assert_eq!(value["source_lang"].as_str(), Some("auto"));
        assert_eq!(value["target_lang"].as_str(), Some("zh-CN"));
        assert_eq!(value["strategy"].as_str(), Some("general"));
    }

    #[test]
    fn retrieval_request_and_events_use_official_field_names() {
        let mut retrieval = RetrievalAgentConfig::new(["kb-1"]);
        retrieval.enable_rerank = true;
        retrieval.similarity_threshold = 0.4;
        let mut request =
            RetrievalAgentRequest::new(retrieval).message(RetrievalAgentMessage::user("question"));
        request.messages.push(RetrievalAgentMessage {
            role: "user".into(),
            content: RetrievalAgentMessageContent::Parts(vec![
                RetrievalAgentContentPart::Text {
                    text: "image".into(),
                },
                RetrievalAgentContentPart::ImageUrl {
                    image_url: RetrievalAgentImageUrl {
                        url: "https://example.com/a.png".into(),
                    },
                },
            ]),
        });
        let value = nextjson::to_value(&request).unwrap();
        assert_eq!(value["model"].as_str(), Some("glm-5v-turbo"));
        assert_eq!(value["retrieval"]["top_k"].as_u64(), Some(8));
        assert_eq!(
            value["messages"][1]["content"][1]["type"].as_str(),
            Some("image_url")
        );
        let event: RetrievalAgentEvent = nextjson::from_value(json!({
            "type":"session_created","sessionId":"session-1","data":"ready"
        }))
        .unwrap();
        assert_eq!(event.kind, RetrievalAgentEventType::SessionCreated);
        assert_eq!(event.session_id.as_deref(), Some("session-1"));
        assert!(event.tool_call().unwrap().is_none());

        let tool_call: RetrievalAgentEvent = nextjson::from_value(json!({
            "type":"tool_call",
            "data":{"callId":"call-1","toolName":"search","arguments":{"query":"Rust"}}
        }))
        .unwrap();
        assert_eq!(tool_call.tool_call().unwrap().unwrap().tool_name, "search");
        let tool_result: RetrievalAgentEvent = nextjson::from_value(json!({
            "type":"tool_result",
            "data":{"callId":"call-1","toolName":"search","result":{"count":1},"status":"success","durationMs":12}
        }))
        .unwrap();
        assert_eq!(tool_result.tool_result().unwrap().unwrap().duration_ms, 12);
        let answer: RetrievalAgentEvent = nextjson::from_value(json!({
            "type":"answer","data":"finished"
        }))
        .unwrap();
        assert_eq!(answer.text(), Some("finished"));
        let error: RetrievalAgentEvent = nextjson::from_value(json!({
            "type":"error","data":{"message":"failed"}
        }))
        .unwrap();
        assert_eq!(error.error().unwrap().unwrap().message, "failed");
        let invalid: RetrievalAgentEvent = nextjson::from_value(json!({
            "type":"tool_call"
        }))
        .unwrap();
        assert!(invalid.tool_call().is_err());
    }

    #[test]
    fn conversation_history_response_is_strongly_typed() {
        let response: AgentConversationResponse = nextjson::from_value(json!({
            "conversation_id":"conversation-1",
            "agent_id":"slides_glm_agent",
            "choices":[{"message":[{"role":"assistant","content":[{
                "type":"file_url","tag_cn":"演示文稿","tag_en":"slides","file_url":"https://example.com/slides.pptx"
            }]}]}]
        }))
        .unwrap();
        let content = &response.choices[0].message[0].content[0];
        assert_eq!(content.kind, "file_url");
        assert_eq!(content.tag_en.as_deref(), Some("slides"));
    }

    #[test]
    fn persona_builds_deterministic_prompt() {
        let prompt = manifest().persona.system_prompt().unwrap();
        assert!(prompt.contains("Identity: Lin"));
        assert!(prompt.contains("Role: technical companion"));
        assert!(prompt.contains("Boundaries: Do not invent tool results"));
    }

    #[test]
    fn manifest_round_trip_contains_reference_but_no_secret() {
        let zhipu_manifest = manifest();
        let json = zhipu_manifest.to_json().unwrap();
        assert!(json.contains("ZHIPU_API_KEY"));
        assert!(!json.contains("key.secret"));
        assert_eq!(AgentManifest::from_json(&json).unwrap(), zhipu_manifest);
        assert!(
            zhipu_manifest
                .build_provider(&StaticResolver("key.secret".into()))
                .is_ok()
        );
        let compatible = AgentProviderDeployment::openai_compatible(
            "deepseek",
            "https://api.example.com/v1",
            "DEEPSEEK_API_KEY",
        );
        let compatible_manifest = manifest().provider(compatible);
        assert!(compatible_manifest.validate().is_ok());
        assert!(
            compatible_manifest
                .build_provider(&StaticResolver("token".into()))
                .is_ok()
        );
        assert!(
            EnvironmentSecretResolver
                .resolve(&CredentialReference::new(
                    "RUSTGLM_TEST_MISSING_CREDENTIAL_9F2A"
                ))
                .is_err()
        );
    }

    #[test]
    fn rejects_invalid_manifests_and_duplicate_tools() {
        let mut invalid = manifest();
        invalid.max_steps = 0;
        assert!(invalid.validate().is_err());
        let provider = Arc::new(MockProvider {
            responses: Mutex::new(VecDeque::new()),
            requests: Mutex::new(Vec::new()),
        });
        let mut runtime = AgentRuntime::new(provider, manifest()).unwrap();
        runtime.register_tool(EchoTool).unwrap();
        assert!(runtime.register_tool(EchoTool).is_err());
        assert!(runtime.register_tool(EmptyNameTool).is_err());

        let mut invalid = manifest();
        invalid.version = 2;
        assert!(invalid.validate().is_err());
        let mut invalid = manifest();
        invalid.model.clear();
        assert!(invalid.validate().is_err());
        let mut invalid = manifest();
        invalid.temperature = 1.1;
        assert!(invalid.validate().is_err());
        let mut invalid = manifest();
        invalid.history = AgentHistoryPolicy::Recent { max_messages: 0 };
        assert!(invalid.validate().is_err());
        assert!(AgentManifest::from_json("not json").is_err());
        assert!(AgentPersona::default().system_prompt().is_err());
    }

    #[tokio::test]
    async fn runtime_executes_tools_and_retains_history() {
        let provider = Arc::new(MockProvider {
            responses: Mutex::new(VecDeque::from([
                response(None, vec![tool_call()]),
                response(Some("done"), Vec::new()),
            ])),
            requests: Mutex::new(Vec::new()),
        });
        let mut runtime = AgentRuntime::new(provider.clone(), manifest()).unwrap();
        runtime.register_tool(EchoTool).unwrap();
        let result = runtime.run("hello").await.unwrap();
        assert_eq!(result.response.text(), Some("done"));
        assert_eq!(result.model_steps, 2);
        assert_eq!(result.tool_executions[0].output, json!({"echoed":"hello"}));
        assert_eq!(runtime.history().len(), 2);
        let requests = provider.requests.lock().unwrap();
        assert_eq!(requests.len(), 2);
        assert_eq!(requests[1].messages.last().unwrap().role, MessageRole::Tool);
    }

    #[tokio::test]
    async fn runtime_enforces_step_limit_and_tool_validation() {
        let mut limited = manifest();
        limited.max_steps = 1;
        let provider = Arc::new(MockProvider {
            responses: Mutex::new(VecDeque::from([response(None, vec![tool_call()])])),
            requests: Mutex::new(Vec::new()),
        });
        let mut runtime = AgentRuntime::new(provider, limited).unwrap();
        runtime.register_tool(EchoTool).unwrap();
        assert!(matches!(
            runtime.run("hello").await,
            Err(SdkError::Agent(_))
        ));

        let provider = Arc::new(MockProvider {
            responses: Mutex::new(VecDeque::from([response(None, vec![tool_call()])])),
            requests: Mutex::new(Vec::new()),
        });
        let mut runtime = AgentRuntime::new(provider, manifest()).unwrap();
        assert!(matches!(runtime.run("hello").await, Err(SdkError::Tool(_))));

        let provider = Arc::new(MockProvider {
            responses: Mutex::new(VecDeque::from([ChatCompletionResponse::default()])),
            requests: Mutex::new(Vec::new()),
        });
        let mut runtime = AgentRuntime::new(provider, manifest()).unwrap();
        assert!(matches!(
            runtime.run("hello").await,
            Err(SdkError::Agent(_))
        ));

        let mut invalid_call = tool_call();
        invalid_call.function.as_mut().unwrap().arguments = "not-json".into();
        let provider = Arc::new(MockProvider {
            responses: Mutex::new(VecDeque::from([response(None, vec![invalid_call])])),
            requests: Mutex::new(Vec::new()),
        });
        let mut runtime = AgentRuntime::new(provider, manifest()).unwrap();
        runtime.register_tool(EchoTool).unwrap();
        assert!(matches!(runtime.run("hello").await, Err(SdkError::Tool(_))));

        let mut missing_function = tool_call();
        missing_function.function = None;
        let provider = Arc::new(MockProvider {
            responses: Mutex::new(VecDeque::from([response(None, vec![missing_function])])),
            requests: Mutex::new(Vec::new()),
        });
        let mut runtime = AgentRuntime::new(provider, manifest()).unwrap();
        assert!(matches!(runtime.run("hello").await, Err(SdkError::Tool(_))));
    }

    #[tokio::test]
    async fn runtime_integrates_semantic_memory_and_clear_operations() {
        let provider = Arc::new(MockProvider {
            responses: Mutex::new(VecDeque::from([response(Some("answer"), Vec::new())])),
            requests: Mutex::new(Vec::new()),
        });
        let memory = Arc::new(RecordingMemory::default());
        let runtime = AgentRuntime::new(provider.clone(), manifest()).unwrap();
        assert!(runtime.semantic_memory(memory.clone(), 0).is_err());
        let mut runtime = AgentRuntime::new(provider.clone(), manifest())
            .unwrap()
            .semantic_memory(memory.clone(), 2)
            .unwrap();
        runtime.run("question").await.unwrap();
        assert_eq!(memory.documents.lock().unwrap().len(), 1);
        {
            let requests = provider.requests.lock().unwrap();
            assert!(requests[0].messages.iter().any(|message| {
                matches!(
                    message.content.as_ref(),
                    Some(MessageContent::Text(value)) if value.contains("remembered context")
                )
            }));
        }
        runtime.clear_history();
        assert!(runtime.history().is_empty());
        runtime.clear_memory().await.unwrap();
        assert!(memory.cleared.load(Ordering::SeqCst));
    }

    #[tokio::test]
    async fn runtime_handles_multimodal_tool_response_content() {
        let mut first = response(None, vec![tool_call()]);
        first.choices[0].message.content =
            Some(ResponseContent::Parts(vec![ResponseContentPart {
                kind: "text".into(),
                text: Some("working".into()),
                extra: Map::new(),
            }]));
        let provider = Arc::new(MockProvider {
            responses: Mutex::new(VecDeque::from([first, response(Some("done"), Vec::new())])),
            requests: Mutex::new(Vec::new()),
        });
        let mut runtime = AgentRuntime::new(provider, manifest()).unwrap();
        runtime.register_tool(EchoTool).unwrap();
        assert_eq!(runtime.run("hello").await.unwrap().model_steps, 2);
    }

    #[test]
    fn sse_decoder_handles_chunks_done_and_errors() {
        let mut decoder = crate::sse::SseDecoder::<RetrievalAgentEvent>::default();
        assert!(decoder.push(b"data: {\"type\":\"ans").unwrap().is_empty());
        let values = decoder
            .push(b"wer\",\"data\":\"ok\"}\n\ndata: [DONE]\n\n")
            .unwrap();
        assert_eq!(values.len(), 1);
        assert_eq!(values[0].kind, RetrievalAgentEventType::Answer);
        assert!(decoder.push(b"data: {}\n\n").unwrap().is_empty());
        let mut invalid = crate::sse::SseDecoder::<RetrievalAgentEvent>::default();
        assert!(invalid.push(b"data: nope\n\n").is_err());
        let mut trailing = crate::sse::SseDecoder::<RetrievalAgentEvent>::default();
        trailing
            .push(b"data: {\"type\":\"answer\",\"data\":\"tail\"}")
            .unwrap();
        assert_eq!(trailing.finish().unwrap()[0].text(), Some("tail"));
    }
}
