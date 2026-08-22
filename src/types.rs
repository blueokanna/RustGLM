use std::marker::PhantomData;

use nextjson::{Map, Value};
use nextjson::{NsonDeserialize as Deserialize, NsonSerialize as Serialize};

use crate::wire_enum;

pub type ExtraFields = Map;

wire_enum! {
    /// The role of a chat message.
    pub enum MessageRole {
        System => "system",
        Developer => "developer",
        User => "user",
        Assistant => "assistant",
        Tool => "tool",
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum MessageContent {
    Text(String),
    Parts(Vec<ContentPart>),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentPart {
    Text { text: String },
    ImageUrl { image_url: MediaUrl },
    VideoUrl { video_url: MediaUrl },
    FileUrl { file_url: MediaUrl },
    InputAudio { input_audio: InputAudio },
}

impl ContentPart {
    pub fn text(value: impl Into<String>) -> Self {
        Self::Text { text: value.into() }
    }

    pub fn image_url(value: impl Into<String>) -> Self {
        Self::ImageUrl {
            image_url: MediaUrl { url: value.into() },
        }
    }

    pub fn video_url(value: impl Into<String>) -> Self {
        Self::VideoUrl {
            video_url: MediaUrl { url: value.into() },
        }
    }

    pub fn file_url(value: impl Into<String>) -> Self {
        Self::FileUrl {
            file_url: MediaUrl { url: value.into() },
        }
    }

    pub fn input_audio(data: impl Into<String>, format: impl Into<String>) -> Self {
        Self::InputAudio {
            input_audio: InputAudio {
                data: data.into(),
                format: format.into(),
            },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MediaUrl {
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct InputAudio {
    #[njson(sensitive)]
    pub data: String,
    pub format: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ChatMessage {
    pub role: MessageRole,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<MessageContent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning_content: Option<String>,
    #[serde(flatten, default)]
    pub extra: ExtraFields,
}

impl ChatMessage {
    pub fn system(value: impl Into<String>) -> Self {
        Self::text(MessageRole::System, value)
    }

    pub fn developer(value: impl Into<String>) -> Self {
        Self::text(MessageRole::Developer, value)
    }

    pub fn user(value: impl Into<String>) -> Self {
        Self::text(MessageRole::User, value)
    }

    pub fn assistant(value: impl Into<String>) -> Self {
        Self::text(MessageRole::Assistant, value)
    }

    pub fn multimodal(role: MessageRole, parts: Vec<ContentPart>) -> Self {
        Self {
            role,
            content: Some(MessageContent::Parts(parts)),
            name: None,
            tool_call_id: None,
            tool_calls: None,
            reasoning_content: None,
            extra: Map::new(),
        }
    }

    pub fn tool_result(tool_call_id: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            role: MessageRole::Tool,
            content: Some(MessageContent::Text(value.into())),
            name: None,
            tool_call_id: Some(tool_call_id.into()),
            tool_calls: None,
            reasoning_content: None,
            extra: Map::new(),
        }
    }

    pub fn assistant_tool_calls(tool_calls: Vec<ToolCall>) -> Self {
        Self {
            role: MessageRole::Assistant,
            content: None,
            name: None,
            tool_call_id: None,
            tool_calls: Some(tool_calls),
            reasoning_content: None,
            extra: Map::new(),
        }
    }

    fn text(role: MessageRole, value: impl Into<String>) -> Self {
        Self {
            role,
            content: Some(MessageContent::Text(value.into())),
            name: None,
            tool_call_id: None,
            tool_calls: None,
            reasoning_content: None,
            extra: Map::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FunctionDefinition {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub parameters: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strict: Option<bool>,
}

impl FunctionDefinition {
    pub fn new(name: impl Into<String>, parameters: Value) -> Self {
        Self {
            name: name.into(),
            description: None,
            parameters,
            strict: None,
        }
    }

    pub fn description(mut self, value: impl Into<String>) -> Self {
        self.description = Some(value.into());
        self
    }

    pub fn strict(mut self, value: bool) -> Self {
        self.strict = Some(value);
        self
    }
}

/// Associates a function's wire definition with its Rust argument and output types.
///
/// Implementations remain open to downstream crates; model capability traits are sealed, while
/// application-owned tools must be extensible.
pub trait FunctionSpec: Send + Sync + 'static {
    type Arguments: for<'de> Deserialize<'de>;
    type Output: Serialize;

    const NAME: &'static str;
    const DESCRIPTION: &'static str;

    fn parameters() -> Value;
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TypedFunction<S: FunctionSpec> {
    marker: PhantomData<fn(S::Arguments) -> S::Output>,
}

impl<S: FunctionSpec> TypedFunction<S> {
    pub const fn new() -> Self {
        Self {
            marker: PhantomData,
        }
    }

    pub fn definition(&self) -> FunctionDefinition {
        FunctionDefinition::new(S::NAME, S::parameters()).description(S::DESCRIPTION)
    }

    pub fn tool(&self) -> Tool {
        Tool::function(self.definition())
    }

    pub fn decode(&self, call: &FunctionCall) -> crate::Result<S::Arguments> {
        if call.name != S::NAME {
            return Err(crate::SdkError::Tool(
                format!("expected function {}, received {}", S::NAME, call.name).into(),
            ));
        }
        call.arguments().map_err(|error| {
            crate::SdkError::Tool(format!("invalid {} arguments: {error}", S::NAME).into())
        })
    }

    pub fn output_message(
        &self,
        call_id: impl Into<String>,
        output: &S::Output,
    ) -> crate::Result<ChatMessage> {
        let output = nextjson::to_string(output).map_err(|error| {
            crate::SdkError::Tool(format!("cannot encode {} output: {error}", S::NAME).into())
        })?;
        Ok(ChatMessage::tool_result(call_id, output))
    }
}

wire_enum! {
    /// Hosted web search engine.
    pub enum WebSearchEngine {
        SearchStd => "search_std",
        SearchPro => "search_pro",
        SearchProSogou => "search_pro_sogou",
        SearchProQuark => "search_pro_quark",
    }
}

wire_enum! {
    /// Web search result recency window.
    pub enum WebSearchRecency {
        OneDay => "oneDay",
        OneWeek => "oneWeek",
        OneMonth => "oneMonth",
        OneYear => "oneYear",
        NoLimit => "noLimit",
    }
}

wire_enum! {
    /// Web search result content size.
    pub enum WebSearchContentSize {
        Medium => "medium",
        High => "high",
    }
}

wire_enum! {
    /// Web search result ordering.
    pub enum WebSearchResultSequence {
        Before => "before",
        After => "after",
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WebSearchTool {
    pub search_engine: WebSearchEngine,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search_query: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search_intent: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub count: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search_domain_filter: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search_recency_filter: Option<WebSearchRecency>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_size: Option<WebSearchContentSize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result_sequence: Option<WebSearchResultSequence>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search_result: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub require_search: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search_prompt: Option<String>,
}

impl WebSearchTool {
    pub fn new(search_engine: WebSearchEngine) -> Self {
        Self {
            search_engine,
            enable: None,
            search_query: None,
            search_intent: None,
            count: None,
            search_domain_filter: None,
            search_recency_filter: None,
            content_size: None,
            result_sequence: None,
            search_result: None,
            require_search: None,
            search_prompt: None,
        }
    }

    pub fn count(mut self, value: u8) -> crate::Result<Self> {
        if !(1..=50).contains(&value) {
            return Err(crate::SdkError::Validation(
                "web search count must be between 1 and 50".into(),
            ));
        }
        self.count = Some(value);
        Ok(self)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RetrievalTool {
    pub knowledge_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_template: Option<String>,
}

impl RetrievalTool {
    pub fn new(knowledge_id: impl Into<String>) -> Self {
        Self {
            knowledge_id: knowledge_id.into(),
            prompt_template: None,
        }
    }

    pub fn prompt_template(mut self, value: impl Into<String>) -> Self {
        self.prompt_template = Some(value.into());
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum McpTransport {
    Sse,
    StreamableHttp,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct McpTool {
    pub server_label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub server_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transport_type: Option<McpTransport>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub allowed_tools: Vec<String>,
    #[serde(default, skip_serializing_if = "Map::is_empty")]
    pub headers: Map,
}

impl McpTool {
    pub fn new(server_label: impl Into<String>) -> Self {
        Self {
            server_label: server_label.into(),
            server_url: None,
            transport_type: None,
            allowed_tools: Vec::new(),
            headers: Map::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Tool {
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(flatten)]
    pub definition: ExtraFields,
}

impl Tool {
    pub fn function(value: FunctionDefinition) -> Self {
        let mut definition = Map::new();
        definition.insert(
            "function".into(),
            nextjson::to_value(&value).unwrap_or(Value::Null),
        );
        Self {
            kind: "function".into(),
            definition,
        }
    }

    pub fn configured(kind: impl Into<String>, key: impl Into<String>, value: Value) -> Self {
        let mut definition = Map::new();
        definition.insert(key.into(), value);
        Self {
            kind: kind.into(),
            definition,
        }
    }

    pub fn web_search(value: WebSearchTool) -> Self {
        Self::configured(
            "web_search",
            "web_search",
            nextjson::to_value(&value).unwrap_or(Value::Null),
        )
    }

    pub fn retrieval(value: RetrievalTool) -> Self {
        Self::configured(
            "retrieval",
            "retrieval",
            nextjson::to_value(&value).unwrap_or(Value::Null),
        )
    }

    pub fn mcp(value: McpTool) -> Self {
        Self::configured(
            "mcp",
            "mcp",
            nextjson::to_value(&value).unwrap_or(Value::Null),
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ToolCall {
    pub id: String,
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub function: Option<FunctionCall>,
    #[serde(flatten, default)]
    pub extra: ExtraFields,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FunctionCall {
    pub name: String,
    pub arguments: String,
}

impl FunctionCall {
    pub fn arguments<T: for<'de> Deserialize<'de>>(&self) -> nextjson::Result<T> {
        nextjson::from_str(&self.arguments)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum ToolChoice {
    Mode(String),
    Function {
        #[serde(rename = "type")]
        kind: String,
        function: ToolChoiceFunction,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ToolChoiceFunction {
    pub name: String,
}

wire_enum! {
    /// Whether thinking is enabled for a request.
    pub enum ThinkingType {
        Enabled => "enabled",
        Disabled => "disabled",
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Thinking {
    #[serde(rename = "type")]
    pub kind: ThinkingType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub clear_thinking: Option<bool>,
}

impl Thinking {
    pub fn enabled() -> Self {
        Self {
            kind: ThinkingType::Enabled,
            clear_thinking: None,
        }
    }

    pub fn disabled() -> Self {
        Self {
            kind: ThinkingType::Disabled,
            clear_thinking: None,
        }
    }

    pub fn clear_thinking(mut self, value: bool) -> Self {
        self.clear_thinking = Some(value);
        self
    }
}

wire_enum! {
    /// Reasoning effort level.
    pub enum ReasoningEffort {
        Max => "max",
        Xhigh => "xhigh",
        High => "high",
        Medium => "medium",
        Low => "low",
        Minimal => "minimal",
        None => "none",
    }
}

wire_enum! {
    /// Response format kind.
    pub enum ResponseFormatType {
        Text => "text",
        JsonObject => "json_object",
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ResponseFormat {
    #[serde(rename = "type")]
    pub kind: ResponseFormatType,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct ChatCompletionRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thinking: Option<Thinking>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning_effort: Option<ReasoningEffort>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub do_sample: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_stream: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<Tool>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<ToolChoice>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_format: Option<ResponseFormat>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub watermark_enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,
    #[serde(flatten, default)]
    pub extra: ExtraFields,
}

impl ChatCompletionRequest {
    pub fn new(model: impl Into<String>) -> Self {
        Self {
            model: model.into(),
            ..Self::default()
        }
    }

    pub fn message(mut self, value: ChatMessage) -> Self {
        self.messages.push(value);
        self
    }

    pub fn messages(mut self, value: impl IntoIterator<Item = ChatMessage>) -> Self {
        self.messages.extend(value);
        self
    }

    pub fn temperature(mut self, value: f32) -> Self {
        self.temperature = Some(value);
        self
    }

    pub fn top_p(mut self, value: f32) -> Self {
        self.top_p = Some(value);
        self
    }

    pub fn max_tokens(mut self, value: u32) -> Self {
        self.max_tokens = Some(value);
        self
    }

    pub fn thinking(mut self, value: Thinking) -> Self {
        self.thinking = Some(value);
        self
    }

    pub fn reasoning_effort(mut self, value: ReasoningEffort) -> Self {
        self.reasoning_effort = Some(value);
        self
    }

    pub fn tool_stream(mut self, value: bool) -> Self {
        self.tool_stream = Some(value);
        self
    }

    pub fn tools(mut self, value: Vec<Tool>) -> Self {
        self.tools = Some(value);
        self
    }

    pub fn watermark_enabled(mut self, value: bool) -> Self {
        self.watermark_enabled = Some(value);
        self
    }

    pub fn request_id(mut self, value: impl Into<String>) -> Self {
        self.request_id = Some(value.into());
        self
    }

    pub fn extra(
        mut self,
        key: impl Into<String>,
        value: impl Serialize,
    ) -> nextjson::Result<Self> {
        self.extra.insert(key.into(), nextjson::to_value(&value)?);
        Ok(self)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct ChatCompletionResponse {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub request_id: Option<String>,
    #[serde(default)]
    pub created: Option<i64>,
    #[serde(default)]
    pub model: String,
    #[serde(default)]
    pub choices: Vec<ChatChoice>,
    #[serde(default)]
    pub usage: Option<Usage>,
    #[serde(default)]
    pub web_search: Vec<WebSearchResult>,
    #[serde(default)]
    pub content_filter: Vec<ContentFilter>,
    #[serde(default)]
    pub video_result: Vec<VideoResult>,
    #[serde(flatten, default)]
    pub extra: ExtraFields,
}

impl ChatCompletionResponse {
    pub fn text(&self) -> Option<&str> {
        self.choices.first()?.message.content.as_ref()?.as_text()
    }

    pub fn joined_text(&self) -> Option<String> {
        self.choices
            .first()?
            .message
            .content
            .as_ref()?
            .joined_text()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ChatChoice {
    #[serde(default)]
    pub index: u32,
    pub message: ChatResponseMessage,
    #[serde(default)]
    pub finish_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct ChatResponseMessage {
    #[serde(default)]
    pub role: Option<MessageRole>,
    #[serde(default)]
    pub content: Option<ResponseContent>,
    #[serde(default)]
    pub reasoning_content: Option<String>,
    #[serde(default)]
    pub tool_calls: Vec<ToolCall>,
    #[serde(default)]
    pub audio: Option<AudioContent>,
    #[serde(flatten, default)]
    pub extra: ExtraFields,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum ResponseContent {
    Text(String),
    Parts(Vec<ResponseContentPart>),
}

impl ResponseContent {
    pub fn as_text(&self) -> Option<&str> {
        match self {
            Self::Text(value) => Some(value),
            Self::Parts(_) => None,
        }
    }

    pub fn joined_text(&self) -> Option<String> {
        match self {
            Self::Text(value) => {
                if value.is_empty() {
                    None
                } else {
                    Some(value.clone())
                }
            }
            Self::Parts(parts) => {
                let mut text = String::new();
                for part in parts {
                    if let Some(value) = part.text.as_deref() {
                        text.push_str(value);
                    }
                }
                if text.is_empty() { None } else { Some(text) }
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ResponseContentPart {
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(default)]
    pub text: Option<String>,
    #[serde(flatten, default)]
    pub extra: ExtraFields,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AudioContent {
    #[serde(default)]
    pub id: Option<String>,
    #[njson(sensitive)]
    #[serde(default)]
    pub data: Option<String>,
    #[serde(default)]
    pub expires_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct ChatCompletionChunk {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub created: Option<i64>,
    #[serde(default)]
    pub model: String,
    #[serde(default)]
    pub choices: Vec<ChatChunkChoice>,
    #[serde(default)]
    pub usage: Option<Usage>,
    #[serde(default)]
    pub content_filter: Vec<ContentFilter>,
    #[serde(flatten, default)]
    pub extra: ExtraFields,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ChatChunkChoice {
    #[serde(default)]
    pub index: u32,
    #[serde(default)]
    pub delta: ChatDelta,
    #[serde(default)]
    pub finish_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct ChatDelta {
    #[serde(default)]
    pub role: Option<MessageRole>,
    #[serde(default)]
    pub content: Option<ResponseContent>,
    #[serde(default)]
    pub reasoning_content: Option<String>,
    #[serde(default)]
    pub tool_calls: Vec<ToolCallDelta>,
    #[serde(default)]
    pub audio: Option<AudioContent>,
    #[serde(flatten, default)]
    pub extra: ExtraFields,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct ToolCallDelta {
    #[serde(default)]
    pub index: Option<u32>,
    #[serde(default)]
    pub id: Option<String>,
    #[serde(rename = "type", default)]
    pub kind: Option<String>,
    #[serde(default)]
    pub function: Option<FunctionCallDelta>,
    #[serde(flatten, default)]
    pub extra: ExtraFields,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct FunctionCallDelta {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub arguments: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct Usage {
    #[serde(default)]
    pub prompt_tokens: u64,
    #[serde(default)]
    pub completion_tokens: u64,
    #[serde(default)]
    pub total_tokens: u64,
    #[serde(default)]
    pub prompt_tokens_details: Option<PromptTokensDetails>,
    #[serde(flatten, default)]
    pub extra: ExtraFields,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct PromptTokensDetails {
    #[serde(default)]
    pub cached_tokens: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct ContentFilter {
    #[serde(default)]
    pub role: String,
    #[serde(default)]
    pub level: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct WebSearchResult {
    #[serde(default)]
    pub icon: Option<String>,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub link: Option<String>,
    #[serde(default)]
    pub media: Option<String>,
    #[serde(default)]
    pub publish_date: Option<String>,
    #[serde(default)]
    pub content: Option<String>,
    #[serde(default)]
    pub refer: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct VideoResult {
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub cover_image_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EmbeddingRequest {
    pub model: String,
    pub input: EmbeddingInput,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dimensions: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub encoding_format: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    #[serde(flatten, default)]
    pub extra: ExtraFields,
}

impl EmbeddingRequest {
    pub fn new(model: impl Into<String>, input: impl Into<EmbeddingInput>) -> Self {
        Self {
            model: model.into(),
            input: input.into(),
            dimensions: None,
            encoding_format: None,
            user_id: None,
            request_id: None,
            extra: Map::new(),
        }
    }

    pub fn dimensions(mut self, value: u32) -> Self {
        self.dimensions = Some(value);
        self
    }

    pub fn encoding_format(mut self, value: impl Into<String>) -> Self {
        self.encoding_format = Some(value.into());
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum EmbeddingInput {
    Text(String),
    Texts(Vec<String>),
}

impl From<String> for EmbeddingInput {
    fn from(value: String) -> Self {
        Self::Text(value)
    }
}

impl From<&str> for EmbeddingInput {
    fn from(value: &str) -> Self {
        Self::Text(value.to_owned())
    }
}

impl From<Vec<String>> for EmbeddingInput {
    fn from(value: Vec<String>) -> Self {
        Self::Texts(value)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct EmbeddingResponse {
    #[serde(default)]
    pub model: String,
    #[serde(default)]
    pub object: String,
    #[serde(default)]
    pub data: Vec<EmbeddingData>,
    #[serde(default)]
    pub usage: Option<Usage>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct EmbeddingData {
    #[serde(default)]
    pub object: Option<String>,
    #[serde(default)]
    pub index: u32,
    #[serde(default)]
    pub embedding: Vec<f32>,
}

pub const GLM_IMAGE_MODEL: &str = "glm-image";
pub const COGVIEW_4_MODEL: &str = "cogview-4";
pub const COGVIEW_3_FLASH_MODEL: &str = "cogview-3-flash";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct ImageGenerationRequest {
    pub model: String,
    pub prompt: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quality: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub watermark_enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,
    #[serde(flatten, default)]
    pub extra: ExtraFields,
}

impl ImageGenerationRequest {
    pub fn new(model: impl Into<String>, prompt: impl Into<String>) -> Self {
        Self {
            model: model.into(),
            prompt: prompt.into(),
            ..Self::default()
        }
    }

    pub fn size(mut self, value: impl Into<String>) -> Self {
        self.size = Some(value.into());
        self
    }

    pub fn quality(mut self, value: impl Into<String>) -> Self {
        self.quality = Some(value.into());
        self
    }

    pub fn watermark(mut self, enabled: bool) -> Self {
        self.watermark_enabled = Some(enabled);
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct ImageGenerationResponse {
    #[serde(default)]
    pub created: Option<i64>,
    #[serde(default)]
    pub data: Vec<GeneratedImage>,
    #[serde(default)]
    pub content_filter: Vec<ContentFilter>,
    #[serde(flatten, default)]
    pub extra: ExtraFields,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct GeneratedImage {
    #[serde(default)]
    pub url: Option<String>,
    #[njson(sensitive)]
    #[serde(default)]
    pub b64_json: Option<String>,
    #[serde(flatten, default)]
    pub extra: ExtraFields,
}

pub const COGVIDEOX_3_MODEL: &str = "cogvideox-3";
pub const COGVIDEOX_FLASH_MODEL: &str = "cogvideox-flash";
pub const VIDU_Q1_MODEL: &str = "vidu-q1";
pub const VIDU_2_MODEL: &str = "vidu-2";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct VideoGenerationRequest {
    pub model: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_url: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quality: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fps: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub with_audio: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub watermark_enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,
    #[serde(flatten, default)]
    pub extra: ExtraFields,
}

impl VideoGenerationRequest {
    pub fn new(model: impl Into<String>) -> Self {
        Self {
            model: model.into(),
            ..Self::default()
        }
    }

    pub fn prompt(mut self, value: impl Into<String>) -> Self {
        self.prompt = Some(value.into());
        self
    }

    pub fn image_url(mut self, value: impl Into<String>) -> Self {
        self.image_url = Some(Value::String(value.into()));
        self
    }

    pub fn quality(mut self, value: impl Into<String>) -> Self {
        self.quality = Some(value.into());
        self
    }

    pub fn size(mut self, value: impl Into<String>) -> Self {
        self.size = Some(value.into());
        self
    }

    pub fn duration(mut self, seconds: u32) -> Self {
        self.duration = Some(seconds);
        self
    }

    pub fn with_audio(mut self, enabled: bool) -> Self {
        self.with_audio = Some(enabled);
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct AsyncTaskResponse {
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub request_id: Option<String>,
    #[serde(default)]
    pub task_status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AsyncTaskResult {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub request_id: Option<String>,
    #[serde(default)]
    pub task_status: String,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub choices: Vec<ChatChoice>,
    #[serde(default)]
    pub video_result: Vec<VideoResult>,
    #[serde(default)]
    pub image_result: Vec<GeneratedImage>,
    #[serde(default)]
    pub usage: Option<Usage>,
    #[serde(flatten, default)]
    pub extra: ExtraFields,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct RerankRequest {
    pub model: String,
    pub query: String,
    pub documents: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_n: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub return_documents: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub return_raw_scores: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,
}

impl RerankRequest {
    pub fn new(
        model: impl Into<String>,
        query: impl Into<String>,
        documents: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        Self {
            model: model.into(),
            query: query.into(),
            documents: documents.into_iter().map(Into::into).collect(),
            ..Self::default()
        }
    }

    pub fn top_n(mut self, value: u32) -> Self {
        self.top_n = Some(value);
        self
    }

    pub fn return_documents(mut self, enabled: bool) -> Self {
        self.return_documents = Some(enabled);
        self
    }

    pub fn return_raw_scores(mut self, enabled: bool) -> Self {
        self.return_raw_scores = Some(enabled);
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct RerankResponse {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub request_id: Option<String>,
    #[serde(default)]
    pub created: Option<i64>,
    #[serde(default)]
    pub results: Vec<RerankResult>,
    #[serde(default)]
    pub usage: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct RerankResult {
    #[serde(default)]
    pub index: u32,
    #[serde(default)]
    pub relevance_score: f64,
    #[serde(default)]
    pub document: Option<String>,
    #[serde(flatten, default)]
    pub extra: ExtraFields,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct TokenizerRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<Tool>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,
}

impl TokenizerRequest {
    pub fn new(model: impl Into<String>, messages: impl IntoIterator<Item = ChatMessage>) -> Self {
        Self {
            model: model.into(),
            messages: messages.into_iter().collect(),
            ..Self::default()
        }
    }

    pub fn tools(mut self, values: impl IntoIterator<Item = Tool>) -> Self {
        self.tools = Some(values.into_iter().collect());
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct TokenizerResponse {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub request_id: Option<String>,
    #[serde(default)]
    pub created: Option<i64>,
    #[serde(default)]
    pub usage: TokenizerUsage,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct TokenizerUsage {
    #[serde(default)]
    pub prompt_tokens: u64,
    #[serde(default)]
    pub video_tokens: u64,
    #[serde(default)]
    pub image_tokens: u64,
    #[serde(default)]
    pub total_tokens: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct SpeechRequest {
    pub model: String,
    pub input: String,
    pub voice: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub speed: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub volume: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_format: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub watermark_enabled: Option<bool>,
    #[serde(flatten, default)]
    pub extra: ExtraFields,
}

impl SpeechRequest {
    pub fn new(
        model: impl Into<String>,
        input: impl Into<String>,
        voice: impl Into<String>,
    ) -> Self {
        Self {
            model: model.into(),
            input: input.into(),
            voice: voice.into(),
            ..Self::default()
        }
    }

    pub fn speed(mut self, value: f32) -> Self {
        self.speed = Some(value);
        self
    }

    pub fn volume(mut self, value: f32) -> Self {
        self.volume = Some(value);
        self
    }

    pub fn response_format(mut self, value: impl Into<String>) -> Self {
        self.response_format = Some(value.into());
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TranscriptionRequest {
    pub model: String,
    pub file_name: String,
    pub file: Vec<u8>,
    pub mime_type: Option<String>,
    pub prompt: Option<String>,
    pub hotwords: Vec<String>,
    pub request_id: Option<String>,
    pub user_id: Option<String>,
}

impl TranscriptionRequest {
    pub fn from_bytes(
        model: impl Into<String>,
        file_name: impl Into<String>,
        file: impl Into<Vec<u8>>,
    ) -> Self {
        Self {
            model: model.into(),
            file_name: file_name.into(),
            file: file.into(),
            mime_type: None,
            prompt: None,
            hotwords: Vec::new(),
            request_id: None,
            user_id: None,
        }
    }

    pub fn mime_type(mut self, value: impl Into<String>) -> Self {
        self.mime_type = Some(value.into());
        self
    }

    pub fn prompt(mut self, value: impl Into<String>) -> Self {
        self.prompt = Some(value.into());
        self
    }

    pub fn hotwords(mut self, values: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.hotwords = values.into_iter().map(Into::into).collect();
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct TranscriptionResponse {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub created: Option<i64>,
    #[serde(default)]
    pub request_id: Option<String>,
    #[serde(default)]
    pub model: String,
    #[serde(default)]
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileUploadRequest {
    pub file_name: String,
    pub file: Vec<u8>,
    pub mime_type: Option<String>,
    pub purpose: String,
}

impl FileUploadRequest {
    pub fn from_bytes(
        file_name: impl Into<String>,
        file: impl Into<Vec<u8>>,
        purpose: impl Into<String>,
    ) -> Self {
        Self {
            file_name: file_name.into(),
            file: file.into(),
            mime_type: None,
            purpose: purpose.into(),
        }
    }

    pub fn mime_type(mut self, value: impl Into<String>) -> Self {
        self.mime_type = Some(value.into());
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct FileObject {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub object: Option<String>,
    #[serde(default)]
    pub bytes: Option<u64>,
    #[serde(default)]
    pub created_at: Option<i64>,
    #[serde(default)]
    pub filename: Option<String>,
    #[serde(default)]
    pub purpose: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct FileList {
    #[serde(default)]
    pub object: Option<String>,
    #[serde(default)]
    pub data: Vec<FileObject>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct DeleteResponse {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub object: Option<String>,
    #[serde(default)]
    pub deleted: bool,
}

wire_enum! {
    /// Batch completion window.
    pub enum BatchCompletionWindow {
        TwentyFourHours => "24h",
    }
}

#[allow(clippy::derivable_impls)]
impl Default for BatchCompletionWindow {
    fn default() -> Self {
        Self::TwentyFourHours
    }
}

wire_enum! {
    /// Batch task status.
    pub enum BatchStatus {
        Validating => "validating",
        InProgress => "in_progress",
        Finalizing => "finalizing",
        Completed => "completed",
        Failed => "failed",
        Expired => "expired",
        Cancelling => "cancelling",
        Cancelled => "cancelled",
        Unknown => "unknown",
        ; _ => Unknown
    }
}

#[allow(clippy::derivable_impls)]
impl Default for BatchStatus {
    fn default() -> Self {
        Self::Unknown
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct BatchCreateRequest {
    pub input_file_id: String,
    pub endpoint: String,
    pub completion_window: BatchCompletionWindow,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Value>,
}

impl BatchCreateRequest {
    pub fn new(input_file_id: impl Into<String>, endpoint: impl Into<String>) -> Self {
        Self {
            input_file_id: input_file_id.into(),
            endpoint: endpoint.into(),
            completion_window: BatchCompletionWindow::TwentyFourHours,
            metadata: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct BatchObject {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub object: Option<String>,
    #[serde(default)]
    pub endpoint: Option<String>,
    #[serde(default)]
    pub status: Option<BatchStatus>,
    #[serde(default)]
    pub input_file_id: Option<String>,
    #[serde(default)]
    pub output_file_id: Option<String>,
    #[serde(default)]
    pub error_file_id: Option<String>,
    #[serde(default)]
    pub created_at: Option<i64>,
    #[serde(default)]
    pub metadata: Option<Value>,
    #[serde(flatten, default)]
    pub extra: ExtraFields,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct BatchList {
    #[serde(default)]
    pub object: Option<String>,
    #[serde(default)]
    pub data: Vec<BatchObject>,
    #[serde(default)]
    pub first_id: Option<String>,
    #[serde(default)]
    pub last_id: Option<String>,
    #[serde(default)]
    pub has_more: bool,
}

pub const MODERATION_MODEL: &str = "moderation";
pub const MODERATION_TEXT_LIMIT: usize = 2_000;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ModerationItem {
    Text { text: String },
    ImageUrl { image_url: MediaUrl },
    AudioUrl { audio_url: MediaUrl },
    VideoUrl { video_url: MediaUrl },
}

impl ModerationItem {
    pub fn text(value: impl Into<String>) -> Self {
        Self::Text { text: value.into() }
    }

    pub fn image_url(value: impl Into<String>) -> Self {
        Self::ImageUrl {
            image_url: MediaUrl { url: value.into() },
        }
    }

    pub fn audio_url(value: impl Into<String>) -> Self {
        Self::AudioUrl {
            audio_url: MediaUrl { url: value.into() },
        }
    }

    pub fn video_url(value: impl Into<String>) -> Self {
        Self::VideoUrl {
            video_url: MediaUrl { url: value.into() },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum ModerationInput {
    Text(String),
    Item(ModerationItem),
    Items(Vec<ModerationItem>),
}

impl ModerationInput {
    pub fn text(value: impl Into<String>) -> Self {
        Self::Text(value.into())
    }

    pub fn item(value: ModerationItem) -> Self {
        Self::Item(value)
    }

    pub fn items(values: impl IntoIterator<Item = ModerationItem>) -> Self {
        Self::Items(values.into_iter().collect())
    }

    pub fn text_content(&self) -> Option<&str> {
        match self {
            Self::Text(value) => Some(value),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ModerationRequest {
    #[serde(default = "default_moderation_model")]
    pub model: String,
    pub input: ModerationInput,
}

impl ModerationRequest {
    pub fn new_text(value: impl Into<String>) -> Self {
        Self {
            model: MODERATION_MODEL.into(),
            input: ModerationInput::text(value),
        }
    }

    pub fn new_item(value: ModerationItem) -> Self {
        Self {
            model: MODERATION_MODEL.into(),
            input: ModerationInput::item(value),
        }
    }

    pub fn new_items(values: impl IntoIterator<Item = ModerationItem>) -> Self {
        Self {
            model: MODERATION_MODEL.into(),
            input: ModerationInput::items(values),
        }
    }
}

fn default_moderation_model() -> String {
    MODERATION_MODEL.into()
}

wire_enum! {
    /// Assessed risk level for moderated content.
    pub enum RiskLevel {
        Pass => "PASS",
        Review => "REVIEW",
        Reject => "REJECT",
        Unknown => "unknown",
        ; _ => Unknown
    }
}

#[allow(clippy::derivable_impls)]
impl Default for RiskLevel {
    fn default() -> Self {
        Self::Pass
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct ModerationResult {
    #[serde(default)]
    pub content_type: Option<String>,
    #[serde(default)]
    pub risk_level: Option<RiskLevel>,
    #[serde(default)]
    pub risk_type: Option<Vec<String>>,
    #[serde(flatten, default)]
    pub extra: ExtraFields,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct ModerationResponse {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub created: Option<u64>,
    #[serde(default)]
    pub request_id: Option<String>,
    #[serde(default)]
    pub result_list: Option<Vec<ModerationResult>>,
    #[serde(default)]
    pub usage: Option<Value>,
    #[serde(flatten, default)]
    pub extra: ExtraFields,
}

fn is_false(value: &bool) -> bool {
    !*value
}

#[cfg(test)]
mod tests {
    use super::*;
    use nextjson::json;
    use nextjson::{NsonDeserialize as Deserialize, NsonSerialize as Serialize};

    #[test]
    fn content_part_constructors_cover_all_modalities() {
        let parts = vec![
            ContentPart::text("text"),
            ContentPart::image_url("https://example.com/image.png"),
            ContentPart::video_url("https://example.com/video.mp4"),
            ContentPart::file_url("https://example.com/file.pdf"),
            ContentPart::input_audio("base64", "wav"),
        ];
        let value = nextjson::to_value(&parts).unwrap();
        assert_eq!(value[0]["type"].as_str(), Some("text"));
        assert_eq!(value[1]["type"].as_str(), Some("image_url"));
        assert_eq!(value[2]["type"].as_str(), Some("video_url"));
        assert_eq!(value[3]["type"].as_str(), Some("file_url"));
        assert_eq!(value[4]["type"].as_str(), Some("input_audio"));
    }

    #[test]
    fn message_constructors_cover_roles_and_tools() {
        assert_eq!(ChatMessage::system("s").role, MessageRole::System);
        assert_eq!(ChatMessage::developer("d").role, MessageRole::Developer);
        assert_eq!(ChatMessage::user("u").role, MessageRole::User);
        assert_eq!(ChatMessage::assistant("a").role, MessageRole::Assistant);
        let tool = ChatMessage::tool_result("call", "result");
        assert_eq!(tool.role, MessageRole::Tool);
        assert_eq!(tool.tool_call_id.as_deref(), Some("call"));
    }

    #[test]
    fn request_builder_sets_common_fields_and_extra() {
        let request = ChatCompletionRequest::new("model")
            .messages([ChatMessage::system("s"), ChatMessage::user("u")])
            .temperature(0.4)
            .top_p(0.8)
            .max_tokens(100)
            .thinking(Thinking::disabled())
            .reasoning_effort(ReasoningEffort::High)
            .tool_stream(true)
            .tools(vec![Tool::configured(
                "web_search",
                "web_search",
                json!({}),
            )])
            .request_id("request-id")
            .extra("custom", 7)
            .unwrap();
        assert_eq!(request.messages.len(), 2);
        assert_eq!(request.temperature, Some(0.4));
        assert_eq!(request.top_p, Some(0.8));
        assert_eq!(request.max_tokens, Some(100));
        assert_eq!(request.reasoning_effort, Some(ReasoningEffort::High));
        assert_eq!(request.tool_stream, Some(true));
        assert_eq!(request.request_id.as_deref(), Some("request-id"));
        assert_eq!(request.extra["custom"].as_u64(), Some(7));
    }

    #[test]
    fn function_definition_and_tool_choice_serialize() {
        let function =
            FunctionDefinition::new("lookup", json!({"type":"object"})).description("lookup data");
        let tool = Tool::function(function);
        let value = nextjson::to_value(&tool).unwrap();
        assert_eq!(
            value["function"]["description"].as_str(),
            Some("lookup data")
        );
        let choice = ToolChoice::Function {
            kind: "function".into(),
            function: ToolChoiceFunction {
                name: "lookup".into(),
            },
        };
        assert_eq!(
            nextjson::to_value(&choice).unwrap()["function"]["name"].as_str(),
            Some("lookup")
        );
    }

    #[derive(Debug, Deserialize, PartialEq)]
    struct Arguments {
        value: u32,
    }

    #[test]
    fn function_arguments_decode_typed_values() {
        let call = FunctionCall {
            name: "f".into(),
            arguments: r#"{"value":9}"#.into(),
        };
        assert_eq!(
            call.arguments::<Arguments>().unwrap(),
            Arguments { value: 9 }
        );
    }

    struct Weather;

    #[derive(Debug, Deserialize, PartialEq)]
    struct WeatherArguments {
        city: String,
    }

    #[derive(Debug, Serialize, PartialEq)]
    struct WeatherOutput {
        temperature: i32,
    }

    impl FunctionSpec for Weather {
        type Arguments = WeatherArguments;
        type Output = WeatherOutput;

        const NAME: &'static str = "weather";
        const DESCRIPTION: &'static str = "Get weather by city";

        fn parameters() -> Value {
            json!({
                "type":"object",
                "properties":{"city":{"type":"string"}},
                "required":["city"]
            })
        }
    }

    #[test]
    fn typed_function_binds_arguments_output_and_wire_definition() {
        let function = TypedFunction::<Weather>::new();
        let tool = nextjson::to_value(&function.tool()).unwrap();
        assert_eq!(tool["type"].as_str(), Some("function"));
        assert_eq!(tool["function"]["name"].as_str(), Some("weather"));

        let call = FunctionCall {
            name: "weather".into(),
            arguments: r#"{"city":"Beijing"}"#.into(),
        };
        assert_eq!(
            function.decode(&call).unwrap(),
            WeatherArguments {
                city: "Beijing".into()
            }
        );
        let message = function
            .output_message("call-1", &WeatherOutput { temperature: 28 })
            .unwrap();
        assert_eq!(message.tool_call_id.as_deref(), Some("call-1"));
    }

    #[test]
    fn typed_hosted_tools_match_official_wire_shapes() {
        let search = Tool::web_search(
            WebSearchTool::new(WebSearchEngine::SearchPro)
                .count(20)
                .unwrap(),
        );
        let retrieval = Tool::retrieval(RetrievalTool::new("knowledge-1"));
        let mcp = Tool::mcp(McpTool::new("server-1"));
        let values = [search, retrieval, mcp]
            .into_iter()
            .map(|tool| nextjson::to_value(&tool).unwrap())
            .collect::<Vec<_>>();

        assert_eq!(
            values[0]["web_search"]["search_engine"].as_str(),
            Some("search_pro")
        );
        assert_eq!(values[0]["web_search"]["count"].as_u64(), Some(20));
        assert_eq!(
            values[1]["retrieval"]["knowledge_id"].as_str(),
            Some("knowledge-1")
        );
        assert_eq!(values[2]["mcp"]["server_label"].as_str(), Some("server-1"));
        assert!(
            WebSearchTool::new(WebSearchEngine::SearchStd)
                .count(0)
                .is_err()
        );
    }

    #[test]
    fn response_helpers_handle_text_parts_and_empty_choices() {
        let text = ResponseContent::Text("answer".into());
        let parts = ResponseContent::Parts(vec![ResponseContentPart {
            kind: "text".into(),
            text: Some("answer".into()),
            extra: Map::new(),
        }]);
        assert_eq!(text.as_text(), Some("answer"));
        assert_eq!(parts.as_text(), None);
        assert_eq!(ChatCompletionResponse::default().text(), None);
        let response: ChatCompletionResponse = nextjson::from_value(json!({
            "choices":[{"index":0,"message":{"content":"answer"}}]
        }))
        .unwrap();
        assert_eq!(response.text(), Some("answer"));
    }

    #[test]
    fn joined_text_handles_parts_and_skips_non_text() {
        let parts = ResponseContent::Parts(vec![
            ResponseContentPart {
                kind: "text".into(),
                text: Some("Hello ".into()),
                extra: Map::new(),
            },
            ResponseContentPart {
                kind: "image_url".into(),
                text: None,
                extra: Map::new(),
            },
            ResponseContentPart {
                kind: "text".into(),
                text: Some("world".into()),
                extra: Map::new(),
            },
        ]);
        assert_eq!(parts.joined_text().as_deref(), Some("Hello world"));
        assert_eq!(ResponseContent::Text("".into()).joined_text(), None);
        assert_eq!(
            ResponseContent::Parts(vec![ResponseContentPart {
                kind: "image_url".into(),
                text: None,
                extra: Map::new(),
            }])
            .joined_text(),
            None
        );
        let response: ChatCompletionResponse = nextjson::from_value(json!({
            "choices":[{"index":0,"message":{"content":[
                {"type":"text","text":"multi"},
                {"type":"image_url","image_url":{"url":"x"}}
            ]}}]
        }))
        .unwrap();
        assert_eq!(response.joined_text().as_deref(), Some("multi"));
        assert_eq!(response.text(), None);
    }

    #[test]
    fn thinking_constructors_match_wire_values() {
        assert_eq!(
            nextjson::to_value(&Thinking::enabled()).unwrap()["type"].as_str(),
            Some("enabled")
        );
        assert_eq!(
            nextjson::to_value(&Thinking::disabled()).unwrap()["type"].as_str(),
            Some("disabled")
        );
    }

    #[test]
    fn endpoint_request_builders_cover_common_options() {
        let embedding = EmbeddingRequest::new("embedding-3", "text")
            .dimensions(1024)
            .encoding_format("float");
        assert!(matches!(embedding.input, EmbeddingInput::Text(_)));
        assert_eq!(embedding.dimensions, Some(1024));
        assert!(matches!(
            EmbeddingInput::from(vec!["one".to_owned()]),
            EmbeddingInput::Texts(_)
        ));

        let image = ImageGenerationRequest::new("cogview", "prompt")
            .size("1024x1024")
            .quality("hd")
            .watermark(false);
        assert_eq!(image.size.as_deref(), Some("1024x1024"));
        assert_eq!(image.watermark_enabled, Some(false));

        let video = VideoGenerationRequest::new("cogvideo")
            .prompt("prompt")
            .image_url("https://example.com/input.png")
            .quality("quality")
            .size("1920x1080")
            .duration(5)
            .with_audio(true);
        assert_eq!(video.duration, Some(5));
        assert_eq!(video.with_audio, Some(true));

        let rerank = RerankRequest::new("rerank", "query", ["one", "two"])
            .top_n(1)
            .return_documents(true)
            .return_raw_scores(true);
        assert_eq!(rerank.documents.len(), 2);
        assert_eq!(rerank.top_n, Some(1));

        let tokenizer = TokenizerRequest::new("glm", [ChatMessage::user("hello")])
            .tools([Tool::configured("test", "test", json!({}))]);
        assert_eq!(tokenizer.tools.unwrap().len(), 1);

        let speech = SpeechRequest::new("glm-tts", "hello", "voice")
            .speed(1.1)
            .volume(0.8)
            .response_format("wav");
        assert_eq!(speech.speed, Some(1.1));
        assert_eq!(speech.response_format.as_deref(), Some("wav"));

        let transcription = TranscriptionRequest::from_bytes("glm-asr", "audio.wav", [1, 2])
            .mime_type("audio/wav")
            .prompt("verbatim")
            .hotwords(["Rust", "GLM"]);
        assert_eq!(transcription.file, vec![1, 2]);
        assert_eq!(transcription.hotwords, ["Rust", "GLM"]);

        let file = FileUploadRequest::from_bytes("input.jsonl", [1, 2], "batch")
            .mime_type("application/jsonl");
        assert_eq!(file.file_name, "input.jsonl");
        assert_eq!(file.mime_type.as_deref(), Some("application/jsonl"));
    }

    #[test]
    fn moderation_request_matches_official_wire_shapes() {
        let text = ModerationRequest::new_text("content to review");
        let value = nextjson::to_value(&text).unwrap();
        assert_eq!(value["model"].as_str(), Some("moderation"));
        assert_eq!(value["input"].as_str(), Some("content to review"));

        let item =
            ModerationRequest::new_item(ModerationItem::image_url("https://example.com/a.png"));
        let value = nextjson::to_value(&item).unwrap();
        assert_eq!(value["input"]["type"].as_str(), Some("image_url"));
        assert_eq!(
            value["input"]["image_url"]["url"].as_str(),
            Some("https://example.com/a.png")
        );

        let batch = ModerationRequest::new_items([
            ModerationItem::text("first"),
            ModerationItem::audio_url("https://example.com/a.mp3"),
            ModerationItem::video_url("https://example.com/a.mp4"),
        ]);
        let value = nextjson::to_value(&batch).unwrap();
        assert_eq!(value["input"].as_array().unwrap().len(), 3);
        assert_eq!(value["input"][1]["type"].as_str(), Some("audio_url"));
        assert_eq!(value["input"][2]["type"].as_str(), Some("video_url"));
    }

    #[test]
    fn moderation_response_decodes_risk_levels() {
        let response: ModerationResponse = nextjson::from_value(json!({
            "id":"mod-1",
            "created":1700000000,
            "request_id":"req-1",
            "result_list":[
                {"content_type":"text","risk_level":"REJECT","risk_type":["violence"]},
                {"content_type":"image_url","risk_level":"PASS"}
            ]
        }))
        .unwrap();
        assert_eq!(response.id.as_deref(), Some("mod-1"));
        let results = response.result_list.unwrap();
        assert_eq!(results[0].risk_level, Some(RiskLevel::Reject));
        assert_eq!(results[1].risk_level, Some(RiskLevel::Pass));
        assert_eq!(results[0].risk_type.as_ref().unwrap()[0], "violence");
        let unknown: ModerationResponse = nextjson::from_value(json!({
            "result_list":[{"risk_level":"NEW_LEVEL"}]
        }))
        .unwrap();
        assert_eq!(
            unknown.result_list.unwrap()[0].risk_level,
            Some(RiskLevel::Unknown)
        );
    }

    #[test]
    fn watermark_and_generation_model_constants_are_consistent() {
        let request = ChatCompletionRequest::new(GLM_IMAGE_MODEL)
            .watermark_enabled(true)
            .message(ChatMessage::user("hi"));
        assert_eq!(request.watermark_enabled, Some(true));
        assert_eq!(GLM_IMAGE_MODEL, "glm-image");
        assert_eq!(COGVIEW_4_MODEL, "cogview-4");
        assert_eq!(COGVIDEOX_3_MODEL, "cogvideox-3");
        assert_eq!(VIDU_2_MODEL, "vidu-2");
        assert_eq!(MODERATION_MODEL, "moderation");
        assert_eq!(MODERATION_TEXT_LIMIT, 2_000);
    }
}
