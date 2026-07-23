use reqwest::StatusCode;
use thiserror::Error;

pub type Result<T> = std::result::Result<T, SdkError>;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("HTTP {status}: {message}")]
pub struct ApiError {
    pub status: StatusCode,
    pub code: Option<String>,
    pub message: String,
    pub request_id: Option<String>,
    pub body: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ConfigurationError {
    #[error("invalid value for {field}: {reason}")]
    InvalidValue { field: &'static str, reason: String },
    #[error("{0}")]
    Message(String),
}

impl From<String> for ConfigurationError {
    fn from(value: String) -> Self {
        Self::Message(value)
    }
}

impl From<&str> for ConfigurationError {
    fn from(value: &str) -> Self {
        Self::Message(value.to_owned())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ValidationError {
    #[error("missing required field {0}")]
    MissingField(&'static str),
    #[error("invalid value for {field}: {reason}")]
    InvalidValue { field: &'static str, reason: String },
    #[error("serialization failed: {0}")]
    Serialization(String),
    #[error("{0}")]
    Message(String),
}

impl ValidationError {
    pub fn contains(&self, pattern: &str) -> bool {
        self.to_string().contains(pattern)
    }
}

impl From<String> for ValidationError {
    fn from(value: String) -> Self {
        Self::Message(value)
    }
}

impl From<&str> for ValidationError {
    fn from(value: &str) -> Self {
        Self::Message(value.to_owned())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum TimeoutError {
    #[error("{operation} timed out")]
    Operation { operation: &'static str },
    #[error("{0}")]
    Message(String),
}

impl From<String> for TimeoutError {
    fn from(value: String) -> Self {
        Self::Message(value)
    }
}

impl From<&str> for TimeoutError {
    fn from(value: &str) -> Self {
        Self::Message(value.to_owned())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum StreamError {
    #[error("stream payload could not be decoded: {0}")]
    Decode(String),
    #[error("stream protocol violation: {0}")]
    Protocol(String),
    #[error("stream channel is closed")]
    Closed,
    #[error("{0}")]
    Message(String),
}

impl From<String> for StreamError {
    fn from(value: String) -> Self {
        Self::Message(value)
    }
}

impl From<&str> for StreamError {
    fn from(value: &str) -> Self {
        Self::Message(value.to_owned())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum UnsupportedError {
    #[error("capability {0} is not supported")]
    Capability(String),
    #[error("{0}")]
    Message(String),
}

impl From<String> for UnsupportedError {
    fn from(value: String) -> Self {
        Self::Message(value)
    }
}

impl From<&str> for UnsupportedError {
    fn from(value: &str) -> Self {
        Self::Message(value.to_owned())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum AgentError {
    #[error("agent response contained no choices")]
    EmptyResponse,
    #[error("agent stopped after {steps} steps")]
    StepLimit { steps: usize },
    #[error("{0}")]
    Message(String),
}

impl From<String> for AgentError {
    fn from(value: String) -> Self {
        Self::Message(value)
    }
}

impl From<&str> for AgentError {
    fn from(value: &str) -> Self {
        Self::Message(value.to_owned())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ToolError {
    #[error("tool {0} is not registered")]
    NotRegistered(String),
    #[error("invalid arguments for tool {tool}: {reason}")]
    InvalidArguments { tool: String, reason: String },
    #[error("tool {tool} failed: {reason}")]
    Execution { tool: String, reason: String },
    #[error("{0}")]
    Message(String),
}

impl From<String> for ToolError {
    fn from(value: String) -> Self {
        Self::Message(value)
    }
}

impl From<&str> for ToolError {
    fn from(value: &str) -> Self {
        Self::Message(value.to_owned())
    }
}

#[cfg(feature = "batch")]
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum BatchError {
    #[error("batch input_file_id and endpoint are required")]
    MissingCreateFields,
    #[error("invalid batch list limit {0}; expected 1..=100")]
    InvalidLimit(u32),
}

#[cfg(feature = "rag")]
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum RagError {
    #[error("knowledge base name cannot be empty")]
    EmptyKnowledgeName,
    #[error("knowledge retrieval requires at least one knowledge base")]
    EmptyKnowledgeIds,
    #[error("invalid RAG pagination: page must be positive and size must be 1..=100")]
    InvalidPagination,
    #[error("invalid RAG field {field}: {reason}")]
    InvalidField { field: &'static str, reason: String },
}

#[cfg(feature = "mcp")]
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum McpClientError {
    #[error("invalid MCP endpoint: {0}")]
    InvalidEndpoint(String),
    #[error("invalid MCP header {0}")]
    InvalidHeader(String),
    #[error("failed to build MCP HTTP client: {0}")]
    ClientBuild(String),
    #[error("MCP initialization failed: {0}")]
    Initialize(String),
    #[error("MCP request failed: {0}")]
    Request(String),
    #[error("MCP shutdown failed: {0}")]
    Shutdown(String),
}

#[derive(Debug, Error)]
pub enum SdkError {
    #[error("invalid configuration: {0}")]
    Configuration(#[from] ConfigurationError),
    #[error("invalid request: {0}")]
    Validation(#[from] ValidationError),
    #[error("transport error: {0}")]
    Transport(#[from] reqwest::Error),
    #[error("operation timed out: {0}")]
    Timeout(#[from] TimeoutError),
    #[error(transparent)]
    Api(#[from] ApiError),
    #[error("response decode error: {message}")]
    Decode { message: String, body: String },
    #[error("stream error: {0}")]
    Stream(#[from] StreamError),
    #[cfg(feature = "realtime")]
    #[error("WebSocket error: {0}")]
    WebSocket(#[from] tokio_tungstenite::tungstenite::Error),
    #[error("unsupported capability: {0}")]
    Unsupported(#[from] UnsupportedError),
    #[error("agent error: {0}")]
    Agent(#[from] AgentError),
    #[error("tool execution error: {0}")]
    Tool(#[from] ToolError),
    #[cfg(feature = "batch")]
    #[error(transparent)]
    Batch(#[from] BatchError),
    #[cfg(feature = "rag")]
    #[error(transparent)]
    Rag(#[from] RagError),
    #[cfg(feature = "mcp")]
    #[error(transparent)]
    Mcp(#[from] McpClientError),
}
