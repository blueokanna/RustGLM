use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use futures_util::{SinkExt, StreamExt};
use nextjson::{Map, Value};
use nextjson::{NsonDeserialize as Deserialize, NsonSerialize as Serialize};
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio::time::timeout;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::http::header::AUTHORIZATION;
use tokio_tungstenite::tungstenite::protocol::WebSocketConfig;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};

use crate::auth::AuthenticationProvider;
use crate::security::{
    DEFAULT_CONNECTION_CLOSE_TIMEOUT, DEFAULT_MAX_WS_FRAME_BYTES, DEFAULT_MAX_WS_MESSAGE_BYTES,
    DEFAULT_WS_WRITE_TIMEOUT, truncate, validate_ws_url,
};
use crate::{Result, SdkError, ZhipuAuthentication};

mod events;
pub use events::*;

pub const ZHIPU_REALTIME_URL: &str = "wss://open.bigmodel.cn/api/paas/v4/realtime";
pub const GLM_REALTIME_MODEL: &str = "glm-realtime";
pub const GLM_REALTIME_FLASH_MODEL: &str = "glm-realtime-flash";
pub const GLM_REALTIME_AIR_MODEL: &str = "glm-realtime-air";

type RealtimeSocket = WebSocketStream<MaybeTlsStream<TcpStream>>;

static EVENT_SEQUENCE: AtomicU64 = AtomicU64::new(1);

#[derive(Clone)]
pub struct RealtimeConfig {
    pub authentication: ZhipuAuthentication,
    pub url: String,
    pub connect_timeout: Duration,
    pub channel_capacity: usize,
    pub allow_insecure: bool,
    pub max_message_bytes: usize,
    pub max_frame_bytes: usize,
}

impl RealtimeConfig {
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            authentication: ZhipuAuthentication::auto(api_key),
            url: ZHIPU_REALTIME_URL.into(),
            connect_timeout: Duration::from_secs(15),
            channel_capacity: 256,
            allow_insecure: false,
            max_message_bytes: DEFAULT_MAX_WS_MESSAGE_BYTES,
            max_frame_bytes: DEFAULT_MAX_WS_FRAME_BYTES,
        }
    }

    pub fn authentication(mut self, value: ZhipuAuthentication) -> Self {
        self.authentication = value;
        self
    }

    pub fn url(mut self, value: impl Into<String>) -> Self {
        self.url = value.into();
        self
    }

    pub fn connect_timeout(mut self, value: Duration) -> Self {
        self.connect_timeout = value;
        self
    }

    pub fn channel_capacity(mut self, value: usize) -> Self {
        self.channel_capacity = value;
        self
    }

    pub fn allow_insecure(mut self, value: bool) -> Self {
        self.allow_insecure = value;
        self
    }

    pub fn max_message_bytes(mut self, value: usize) -> Self {
        self.max_message_bytes = value;
        self
    }

    pub fn max_frame_bytes(mut self, value: usize) -> Self {
        self.max_frame_bytes = value;
        self
    }

    pub async fn connect(self) -> Result<RealtimeConnection> {
        RealtimeClient::from_config(self).await
    }
}

pub struct RealtimeClient;

impl RealtimeClient {
    pub async fn connect(api_key: impl Into<String>) -> Result<RealtimeConnection> {
        RealtimeConfig::new(api_key).connect().await
    }

    pub async fn from_config(config: RealtimeConfig) -> Result<RealtimeConnection> {
        validate_ws_url(&config.url, config.allow_insecure)?;
        if config.connect_timeout.is_zero() || config.channel_capacity == 0 {
            return Err(SdkError::Configuration(
                "realtime timeout and channel capacity must be greater than zero".into(),
            ));
        }
        if config.max_message_bytes == 0 || config.max_frame_bytes == 0 {
            return Err(SdkError::Configuration(
                "realtime message and frame limits must be greater than zero".into(),
            ));
        }
        let authentication = AuthenticationProvider::zhipu(config.authentication)?;
        let authorization = authentication.header_value()?;
        let mut request = config.url.into_client_request()?;
        request.headers_mut().insert(
            AUTHORIZATION,
            authorization
                .to_str()
                .map_err(|_| SdkError::Configuration("authentication header is invalid".into()))?
                .parse()
                .map_err(|_| SdkError::Configuration("authentication header is invalid".into()))?,
        );
        let websocket = WebSocketConfig::default();
        let mut websocket = websocket;
        websocket.max_message_size = Some(config.max_message_bytes);
        websocket.max_frame_size = Some(config.max_frame_bytes);
        let (socket, _) = timeout(
            config.connect_timeout,
            tokio_tungstenite::connect_async_with_config(request, Some(websocket), false),
        )
        .await
        .map_err(|_| SdkError::Timeout("realtime connection timed out".into()))??;
        Ok(spawn_connection(socket, config.channel_capacity))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RealtimeSession {
    pub model: String,
    pub modalities: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instructions: Option<String>,
    pub voice: String,
    pub input_audio_format: String,
    pub output_audio_format: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_audio_noise_reduction: Option<RealtimeNoiseReduction>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub turn_detection: Option<RealtimeTurnDetection>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(rename = "max_output_tokens", skip_serializing_if = "Option::is_none")]
    pub max_response_output_tokens: Option<RealtimeMaxTokens>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tools: Vec<RealtimeTool>,
    pub beta_fields: RealtimeBetaFields,
    #[serde(flatten, default)]
    pub extra: Map,
}

impl Default for RealtimeSession {
    fn default() -> Self {
        Self {
            model: GLM_REALTIME_MODEL.into(),
            modalities: vec!["text".into(), "audio".into()],
            instructions: None,
            voice: "tongtong".into(),
            input_audio_format: "pcm16".into(),
            output_audio_format: "pcm".into(),
            input_audio_noise_reduction: None,
            turn_detection: None,
            temperature: None,
            max_response_output_tokens: None,
            tools: Vec::new(),
            beta_fields: RealtimeBetaFields::default(),
            extra: Map::new(),
        }
    }
}

impl RealtimeSession {
    pub fn model(mut self, value: impl Into<String>) -> Self {
        self.model = value.into();
        self
    }

    pub fn instructions(mut self, value: impl Into<String>) -> Self {
        self.instructions = Some(value.into());
        self
    }

    pub fn voice(mut self, value: impl Into<String>) -> Self {
        self.voice = value.into();
        self
    }

    pub fn input_audio_format(mut self, value: impl Into<String>) -> Self {
        self.input_audio_format = value.into();
        self
    }

    pub fn server_vad(mut self, create_response: bool, interrupt_response: bool) -> Self {
        self.turn_detection = Some(RealtimeTurnDetection {
            kind: "server_vad".into(),
            create_response: Some(create_response),
            interrupt_response: Some(interrupt_response),
            prefix_padding_ms: None,
            silence_duration_ms: None,
            threshold: None,
        });
        self
    }

    pub fn video(mut self) -> Self {
        self.beta_fields.chat_mode = "video_passive".into();
        self
    }

    pub fn tool(mut self, value: RealtimeTool) -> Self {
        self.tools.push(value);
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RealtimeNoiseReduction {
    #[serde(rename = "type")]
    pub kind: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RealtimeTurnDetection {
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub create_response: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub interrupt_response: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prefix_padding_ms: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub silence_duration_ms: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub threshold: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RealtimeTranscriptionSession {
    pub input_audio_format: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_audio_noise_reduction: Option<RealtimeNoiseReduction>,
    pub modalities: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub turn_detection: Option<RealtimeTurnDetection>,
}

impl Default for RealtimeTranscriptionSession {
    fn default() -> Self {
        Self {
            input_audio_format: "pcm".into(),
            input_audio_noise_reduction: None,
            modalities: vec!["text".into(), "audio".into()],
            turn_detection: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum RealtimeMaxTokens {
    Count(u16),
    Unlimited(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RealtimeTool {
    #[serde(rename = "type")]
    pub kind: String,
    pub name: String,
    pub description: String,
    pub parameters: Value,
}

impl RealtimeTool {
    pub fn function(
        name: impl Into<String>,
        description: impl Into<String>,
        parameters: Value,
    ) -> Self {
        Self {
            kind: "function".into(),
            name: name.into(),
            description: description.into(),
            parameters,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RealtimeBetaFields {
    pub chat_mode: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tts_source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_search: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub greeting_config: Option<RealtimeGreetingConfig>,
}

impl Default for RealtimeBetaFields {
    fn default() -> Self {
        Self {
            chat_mode: "audio".into(),
            tts_source: None,
            auto_search: None,
            greeting_config: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RealtimeGreetingConfig {
    pub enable: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RealtimeContentPart {
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audio: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transcript: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RealtimeConversationItem {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(rename = "type")]
    pub kind: String,
    pub object: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub content: Vec<RealtimeContentPart>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<String>,
}

impl RealtimeConversationItem {
    pub fn text(role: impl Into<String>, text: impl Into<String>) -> Self {
        Self {
            id: None,
            kind: "message".into(),
            object: "realtime.item".into(),
            status: Some("completed".into()),
            role: Some(role.into()),
            content: vec![RealtimeContentPart {
                kind: "input_text".into(),
                text: Some(text.into()),
                audio: None,
                transcript: None,
            }],
            name: None,
            arguments: None,
            output: None,
        }
    }

    pub fn function_output(output: impl Into<String>) -> Self {
        Self {
            id: None,
            kind: "function_call_output".into(),
            object: "realtime.item".into(),
            status: Some("completed".into()),
            role: None,
            content: Vec::new(),
            name: None,
            arguments: None,
            output: Some(output.into()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum RealtimeClientEvent {
    #[serde(rename = "session.update")]
    SessionUpdate {
        #[serde(flatten)]
        metadata: RealtimeEventMetadata,
        session: Box<RealtimeSession>,
    },
    #[serde(rename = "transcription_session.update")]
    TranscriptionSessionUpdate {
        #[serde(flatten)]
        metadata: RealtimeEventMetadata,
        session: Box<RealtimeTranscriptionSession>,
    },
    #[serde(rename = "input_audio_buffer.append")]
    InputAudioBufferAppend {
        #[serde(flatten)]
        metadata: RealtimeEventMetadata,
        audio: String,
    },
    #[serde(rename = "input_audio_buffer.append_video_frame")]
    InputAudioBufferAppendVideoFrame {
        #[serde(flatten)]
        metadata: RealtimeEventMetadata,
        video_frame: String,
    },
    #[serde(rename = "input_audio_buffer.commit")]
    InputAudioBufferCommit {
        #[serde(flatten)]
        metadata: RealtimeEventMetadata,
    },
    #[serde(rename = "input_audio_buffer.clear")]
    InputAudioBufferClear {
        #[serde(flatten)]
        metadata: RealtimeEventMetadata,
    },
    #[serde(rename = "conversation.item.create")]
    ConversationItemCreate {
        #[serde(flatten)]
        metadata: RealtimeEventMetadata,
        item: Box<RealtimeConversationItem>,
    },
    #[serde(rename = "conversation.item.delete")]
    ConversationItemDelete {
        #[serde(flatten)]
        metadata: RealtimeEventMetadata,
        item_id: String,
    },
    #[serde(rename = "conversation.item.retrieve")]
    ConversationItemRetrieve {
        #[serde(flatten)]
        metadata: RealtimeEventMetadata,
        item_id: String,
    },
    #[serde(rename = "response.create")]
    ResponseCreate {
        #[serde(flatten)]
        metadata: RealtimeEventMetadata,
    },
    #[serde(rename = "response.cancel")]
    ResponseCancel {
        #[serde(flatten)]
        metadata: RealtimeEventMetadata,
    },
}

impl RealtimeClientEvent {
    pub fn session_update(session: RealtimeSession) -> Result<Self> {
        Ok(Self::SessionUpdate {
            metadata: RealtimeEventMetadata::new()?,
            session: Box::new(session),
        })
    }

    pub fn append_audio(bytes: &[u8]) -> Result<Self> {
        Self::append_audio_base64(STANDARD.encode(bytes))
    }

    pub fn transcription_session_update(session: RealtimeTranscriptionSession) -> Result<Self> {
        Ok(Self::TranscriptionSessionUpdate {
            metadata: RealtimeEventMetadata::new()?,
            session: Box::new(session),
        })
    }

    pub fn append_audio_base64(value: impl Into<String>) -> Result<Self> {
        let audio = value.into();
        if audio.is_empty() {
            return Err(SdkError::Validation("audio data cannot be empty".into()));
        }
        Ok(Self::InputAudioBufferAppend {
            metadata: RealtimeEventMetadata::new()?,
            audio,
        })
    }

    pub fn append_video_frame(jpeg: &[u8]) -> Result<Self> {
        if jpeg.is_empty() {
            return Err(SdkError::Validation("video frame cannot be empty".into()));
        }
        Ok(Self::InputAudioBufferAppendVideoFrame {
            metadata: RealtimeEventMetadata::new()?,
            video_frame: STANDARD.encode(jpeg),
        })
    }

    pub fn commit() -> Result<Self> {
        Ok(Self::InputAudioBufferCommit {
            metadata: RealtimeEventMetadata::new()?,
        })
    }

    pub fn clear() -> Result<Self> {
        Ok(Self::InputAudioBufferClear {
            metadata: RealtimeEventMetadata::new()?,
        })
    }

    pub fn create_item(item: RealtimeConversationItem) -> Result<Self> {
        Ok(Self::ConversationItemCreate {
            metadata: RealtimeEventMetadata::new()?,
            item: Box::new(item),
        })
    }

    pub fn delete_item(item_id: impl Into<String>) -> Result<Self> {
        let item_id = require_value(item_id.into(), "item id")?;
        Ok(Self::ConversationItemDelete {
            metadata: RealtimeEventMetadata::new()?,
            item_id,
        })
    }

    pub fn retrieve_item(item_id: impl Into<String>) -> Result<Self> {
        let item_id = require_value(item_id.into(), "item id")?;
        Ok(Self::ConversationItemRetrieve {
            metadata: RealtimeEventMetadata::new()?,
            item_id,
        })
    }

    pub fn create_response() -> Result<Self> {
        Ok(Self::ResponseCreate {
            metadata: RealtimeEventMetadata::new()?,
        })
    }

    pub fn cancel_response() -> Result<Self> {
        Ok(Self::ResponseCancel {
            metadata: RealtimeEventMetadata::new()?,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RealtimeEventMetadata {
    pub event_id: String,
    pub client_timestamp: u64,
}

impl RealtimeEventMetadata {
    pub fn new() -> Result<Self> {
        let client_timestamp = unix_millis()?;
        let sequence = EVENT_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        Ok(Self {
            event_id: format!("rustglm-{client_timestamp}-{sequence}"),
            client_timestamp,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RealtimeServerEvent {
    #[serde(rename = "type")]
    pub event_type: String,
    #[serde(default)]
    pub event_id: Option<String>,
    #[serde(default)]
    pub client_timestamp: Option<u64>,
    #[serde(flatten, default)]
    pub data: Map,
}

impl RealtimeServerEvent {
    pub fn delta_text(&self) -> Option<&str> {
        matches!(
            self.event_type.as_str(),
            "response.text.delta" | "response.audio_transcript.delta"
        )
        .then(|| self.data.get("delta").and_then(Value::as_str))
        .flatten()
    }

    pub fn audio_base64(&self) -> Option<&str> {
        (self.event_type == "response.audio.delta")
            .then(|| self.data.get("delta").and_then(Value::as_str))
            .flatten()
    }

    pub fn audio_bytes(&self) -> Result<Option<Vec<u8>>> {
        self.audio_base64()
            .map(|value| {
                STANDARD
                    .decode(value)
                    .map_err(|error| SdkError::Stream(error.to_string().into()))
            })
            .transpose()
    }

    pub fn error(&self) -> Option<&Value> {
        (self.event_type == "error")
            .then(|| self.data.get("error"))
            .flatten()
    }

    pub fn function_call(&self) -> Option<RealtimeFunctionCall<'_>> {
        if self.event_type != "response.function_call_arguments.done" {
            return None;
        }
        Some(RealtimeFunctionCall {
            name: self.data.get("name")?.as_str()?,
            arguments: self.data.get("arguments")?.as_str()?,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RealtimeFunctionCall<'a> {
    pub name: &'a str,
    pub arguments: &'a str,
}

enum RealtimeCommand {
    Send(Message),
    Close,
}

#[derive(Clone)]
pub struct RealtimeSender {
    commands: mpsc::Sender<RealtimeCommand>,
}

impl RealtimeSender {
    pub async fn send(&self, event: &RealtimeClientEvent) -> Result<()> {
        let value = nextjson::to_string(event)
            .map_err(|error| SdkError::Validation(error.to_string().into()))?;
        self.send_message(Message::Text(value.into())).await
    }

    pub async fn send_json(&self, event: &Value) -> Result<()> {
        self.send_message(Message::Text(event.to_string().into()))
            .await
    }

    pub async fn send_request(&self, event: &RealtimeRequest) -> Result<()> {
        let value = nextjson::to_string(event)
            .map_err(|error| SdkError::Validation(error.to_string().into()))?;
        self.send_message(Message::Text(value.into())).await
    }

    pub async fn update_typed_session(&self, session: TypedRealtimeSession) -> Result<()> {
        self.send_request(&RealtimeRequest::session_update(session)?)
            .await
    }

    pub async fn create_typed_item(
        &self,
        previous_item_id: Option<String>,
        item: TypedRealtimeItem,
    ) -> Result<()> {
        self.send_request(&RealtimeRequest::create_item(previous_item_id, item)?)
            .await
    }

    pub async fn create_response_with(&self, options: RealtimeResponseOptions) -> Result<()> {
        self.send_request(&RealtimeRequest::create_response(Some(options))?)
            .await
    }

    pub async fn update_session(&self, session: RealtimeSession) -> Result<()> {
        self.send(&RealtimeClientEvent::session_update(session)?)
            .await
    }

    pub async fn append_audio(&self, bytes: &[u8]) -> Result<()> {
        self.send(&RealtimeClientEvent::append_audio(bytes)?).await
    }

    pub async fn append_audio_base64(&self, value: impl Into<String>) -> Result<()> {
        self.send(&RealtimeClientEvent::append_audio_base64(value)?)
            .await
    }

    pub async fn append_video_frame(&self, jpeg: &[u8]) -> Result<()> {
        self.send(&RealtimeClientEvent::append_video_frame(jpeg)?)
            .await
    }

    pub async fn commit(&self) -> Result<()> {
        self.send(&RealtimeClientEvent::commit()?).await
    }

    pub async fn clear_audio(&self) -> Result<()> {
        self.send(&RealtimeClientEvent::clear()?).await
    }

    pub async fn update_transcription_session(
        &self,
        session: RealtimeTranscriptionSession,
    ) -> Result<()> {
        self.send(&RealtimeClientEvent::transcription_session_update(session)?)
            .await
    }

    pub async fn create_item(&self, item: RealtimeConversationItem) -> Result<()> {
        self.send(&RealtimeClientEvent::create_item(item)?).await
    }

    pub async fn delete_item(&self, item_id: impl Into<String>) -> Result<()> {
        self.send(&RealtimeClientEvent::delete_item(item_id)?).await
    }

    pub async fn retrieve_item(&self, item_id: impl Into<String>) -> Result<()> {
        self.send(&RealtimeClientEvent::retrieve_item(item_id)?)
            .await
    }

    pub async fn create_response(&self) -> Result<()> {
        self.send(&RealtimeClientEvent::create_response()?).await
    }

    pub async fn cancel_response(&self) -> Result<()> {
        self.send(&RealtimeClientEvent::cancel_response()?).await
    }

    pub async fn close(&self) -> Result<()> {
        self.commands
            .send(RealtimeCommand::Close)
            .await
            .map_err(|_| SdkError::Stream("realtime connection is closed".into()))
    }

    async fn send_message(&self, message: Message) -> Result<()> {
        self.commands
            .send(RealtimeCommand::Send(message))
            .await
            .map_err(|_| SdkError::Stream("realtime connection is closed".into()))
    }
}

pub struct RealtimeReceiver {
    events: mpsc::Receiver<Result<RealtimeServerEvent>>,
}

impl RealtimeReceiver {
    pub async fn next_event(&mut self) -> Option<Result<RealtimeServerEvent>> {
        self.events.recv().await
    }

    pub async fn next_typed_event(&mut self) -> Option<Result<RealtimeServerMessage>> {
        self.next_event()
            .await
            .map(|event| event.map(RealtimeServerEvent::into_typed))
    }
}

pub struct RealtimeConnection {
    sender: RealtimeSender,
    receiver: RealtimeReceiver,
    task: JoinHandle<()>,
}

impl RealtimeConnection {
    pub fn sender(&self) -> RealtimeSender {
        self.sender.clone()
    }

    pub async fn send(&self, event: &RealtimeClientEvent) -> Result<()> {
        self.sender.send(event).await
    }

    pub async fn next_event(&mut self) -> Option<Result<RealtimeServerEvent>> {
        self.receiver.next_event().await
    }

    pub async fn next_typed_event(&mut self) -> Option<Result<RealtimeServerMessage>> {
        self.receiver.next_typed_event().await
    }

    pub async fn send_request(&self, event: &RealtimeRequest) -> Result<()> {
        self.sender.send_request(event).await
    }

    pub fn split(self) -> (RealtimeSender, RealtimeReceiver) {
        (self.sender, self.receiver)
    }

    pub async fn close(self) -> Result<()> {
        self.sender.close().await?;
        timeout(DEFAULT_CONNECTION_CLOSE_TIMEOUT, self.task)
            .await
            .map_err(|_| SdkError::Timeout("realtime connection close timed out".into()))?
            .map_err(|error| SdkError::Stream(error.to_string().into()))?;
        Ok(())
    }
}

fn spawn_connection(socket: RealtimeSocket, capacity: usize) -> RealtimeConnection {
    let (commands_tx, mut commands_rx) = mpsc::channel(capacity);
    let (events_tx, events_rx) = mpsc::channel(capacity);
    let task = tokio::spawn(async move {
        let (mut sink, mut stream) = socket.split();
        let mut overflow_notified = false;
        let push_event =
            |event: Result<RealtimeServerEvent>, overflow_notified: &mut bool| -> bool {
                match events_tx.try_send(event) {
                    Ok(()) => {
                        *overflow_notified = false;
                        true
                    }
                    Err(mpsc::error::TrySendError::Full(_)) => {
                        if !*overflow_notified {
                            *overflow_notified = true;
                            let _ = events_tx.try_send(Err(SdkError::Stream(
                                "realtime event queue overflow; events were dropped".into(),
                            )));
                        }
                        true
                    }
                    Err(mpsc::error::TrySendError::Closed(_)) => false,
                }
            };
        loop {
            tokio::select! {
                command = commands_rx.recv() => match command {
                    Some(RealtimeCommand::Send(message)) => {
                        match timeout(DEFAULT_WS_WRITE_TIMEOUT, sink.send(message)).await {
                            Ok(Ok(())) => {}
                            Ok(Err(error)) => {
                                push_event(Err(error.into()), &mut overflow_notified);
                                break;
                            }
                            Err(_) => {
                                push_event(
                                    Err(SdkError::Timeout(
                                        "realtime socket write timed out".into(),
                                    )),
                                    &mut overflow_notified,
                                );
                                break;
                            }
                        }
                    }
                    Some(RealtimeCommand::Close) | None => {
                        let _ = sink.close().await;
                        break;
                    }
                },
                message = stream.next() => match message {
                    Some(Ok(Message::Text(text))) => {
                        let event = nextjson::from_str(&text).map_err(|error| {
                            SdkError::Stream(
                                format!("{error}: {}", truncate(&text, 512)).into(),
                            )
                        });
                        if !push_event(event, &mut overflow_notified) {
                            break;
                        }
                    }
                    Some(Ok(Message::Binary(bytes))) => {
                        let event = nextjson::from_slice(&bytes).map_err(|error| {
                            SdkError::Stream(
                                format!("{error}: binary payload of {} bytes", bytes.len()).into(),
                            )
                        });
                        if !push_event(event, &mut overflow_notified) {
                            break;
                        }
                    }
                    Some(Ok(Message::Ping(bytes))) => {
                        match timeout(DEFAULT_WS_WRITE_TIMEOUT, sink.send(Message::Pong(bytes)))
                            .await
                        {
                            Ok(Ok(())) => {}
                            Ok(Err(error)) => {
                                push_event(Err(error.into()), &mut overflow_notified);
                                break;
                            }
                            Err(_) => {
                                push_event(
                                    Err(SdkError::Timeout(
                                        "realtime pong write timed out".into(),
                                    )),
                                    &mut overflow_notified,
                                );
                                break;
                            }
                        }
                    }
                    Some(Ok(Message::Close(_))) | None => break,
                    Some(Ok(_)) => {}
                    Some(Err(error)) => {
                        push_event(Err(error.into()), &mut overflow_notified);
                        break;
                    }
                }
            }
        }
    });
    RealtimeConnection {
        sender: RealtimeSender {
            commands: commands_tx,
        },
        receiver: RealtimeReceiver { events: events_rx },
        task,
    }
}

fn unix_millis() -> Result<u64> {
    let value = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| SdkError::Configuration("system clock is before Unix epoch".into()))?
        .as_millis();
    u64::try_from(value)
        .map_err(|_| SdkError::Configuration("timestamp exceeds supported range".into()))
}

fn require_value(value: String, name: &str) -> Result<String> {
    if value.trim().is_empty() {
        return Err(SdkError::Validation(
            format!("{name} cannot be empty").into(),
        ));
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use tokio::net::TcpListener;
    use tokio_tungstenite::accept_hdr_async;
    use tokio_tungstenite::tungstenite::handshake::server::{Request, Response};

    use super::*;

    #[test]
    fn serializes_official_client_events() {
        let mut session = RealtimeSession::default()
            .model("glm-realtime-flash")
            .instructions("concise")
            .voice("xiaochen")
            .input_audio_format("pcm24")
            .server_vad(true, true)
            .video()
            .tool(RealtimeTool::function(
                "weather",
                "weather lookup",
                nextjson::json!({"type":"object"}),
            ));
        session.max_response_output_tokens = Some(RealtimeMaxTokens::Count(1024));
        let value =
            nextjson::to_value(&RealtimeClientEvent::session_update(session).unwrap()).unwrap();
        assert_eq!(value["type"].as_str(), Some("session.update"));
        assert_eq!(
            value["session"]["beta_fields"]["chat_mode"].as_str(),
            Some("video_passive")
        );
        assert_eq!(
            value["session"]["turn_detection"]["type"].as_str(),
            Some("server_vad")
        );
        assert_eq!(
            value["session"]["tools"][0]["name"].as_str(),
            Some("weather")
        );
        assert_eq!(value["session"]["max_output_tokens"].as_u64(), Some(1024));
        assert!(value["session"].get("max_response_output_tokens").is_none());

        let audio =
            nextjson::to_value(&RealtimeClientEvent::append_audio(&[1, 2]).unwrap()).unwrap();
        assert_eq!(audio["type"].as_str(), Some("input_audio_buffer.append"));
        assert_eq!(audio["audio"].as_str(), Some("AQI="));
        let frame =
            nextjson::to_value(&RealtimeClientEvent::append_video_frame(&[0xff, 0xd8]).unwrap())
                .unwrap();
        assert_eq!(
            frame["type"].as_str(),
            Some("input_audio_buffer.append_video_frame")
        );
        let transcription = nextjson::to_value(
            &RealtimeClientEvent::transcription_session_update(
                RealtimeTranscriptionSession::default(),
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(
            transcription["type"].as_str(),
            Some("transcription_session.update")
        );
        assert_eq!(
            transcription["session"]["input_audio_format"].as_str(),
            Some("pcm")
        );
        let text_item = RealtimeConversationItem::text("user", "hello");
        assert_eq!(text_item.content[0].kind, "input_text");
        let output = RealtimeConversationItem::function_output("{\"ok\":true}");
        assert_eq!(output.kind, "function_call_output");
        for event in [
            RealtimeClientEvent::clear().unwrap(),
            RealtimeClientEvent::create_item(text_item).unwrap(),
            RealtimeClientEvent::delete_item("item-1").unwrap(),
            RealtimeClientEvent::retrieve_item("item-1").unwrap(),
            RealtimeClientEvent::commit().unwrap(),
            RealtimeClientEvent::create_response().unwrap(),
            RealtimeClientEvent::cancel_response().unwrap(),
        ] {
            assert!(nextjson::to_value(&event).unwrap()["type"].is_string());
        }
        assert!(RealtimeClientEvent::append_audio(&[]).is_err());
        assert!(RealtimeClientEvent::append_video_frame(&[]).is_err());
        assert!(RealtimeClientEvent::delete_item("").is_err());
        assert!(RealtimeClientEvent::retrieve_item(" ").is_err());
    }

    #[test]
    fn decodes_server_event_helpers() {
        let text: RealtimeServerEvent = nextjson::from_value(nextjson::json!({
            "type":"response.text.delta","delta":"hello"
        }))
        .unwrap();
        assert_eq!(text.delta_text(), Some("hello"));
        let transcript: RealtimeServerEvent = nextjson::from_value(nextjson::json!({
            "type":"response.audio_transcript.delta","delta":"words"
        }))
        .unwrap();
        assert_eq!(transcript.delta_text(), Some("words"));
        let audio: RealtimeServerEvent = nextjson::from_value(nextjson::json!({
            "type":"response.audio.delta","delta":"AQI="
        }))
        .unwrap();
        assert_eq!(audio.audio_bytes().unwrap(), Some(vec![1, 2]));
        let call: RealtimeServerEvent = nextjson::from_value(nextjson::json!({
            "type":"response.function_call_arguments.done","name":"weather","arguments":"{}"
        }))
        .unwrap();
        assert_eq!(call.function_call().unwrap().name, "weather");
        let error: RealtimeServerEvent = nextjson::from_value(nextjson::json!({
            "type":"error","error":{"code":"bad"}
        }))
        .unwrap();
        assert_eq!(error.error().unwrap()["code"].as_str(), Some("bad"));
        assert!(text.audio_base64().is_none());
        assert!(text.audio_bytes().unwrap().is_none());
        assert!(text.error().is_none());
        assert!(text.function_call().is_none());
        let invalid_audio: RealtimeServerEvent = nextjson::from_value(nextjson::json!({
            "type":"response.audio.delta","delta":"%%%"
        }))
        .unwrap();
        assert!(invalid_audio.audio_bytes().is_err());
    }

    #[tokio::test]
    async fn rejects_invalid_realtime_configuration() {
        assert!(
            RealtimeConfig::new("key")
                .url("https://example.com")
                .connect()
                .await
                .is_err()
        );
        assert!(
            RealtimeConfig::new("key")
                .url("ws://127.0.0.1:1")
                .connect_timeout(Duration::ZERO)
                .connect()
                .await
                .is_err()
        );
        assert!(
            RealtimeConfig::new("key")
                .url("ws://127.0.0.1:1")
                .channel_capacity(0)
                .connect()
                .await
                .is_err()
        );
    }

    #[tokio::test]
    #[allow(clippy::result_large_err)]
    async fn connects_sends_media_and_receives_stream_events() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let authorization = Arc::new(Mutex::new(String::new()));
        let captured = authorization.clone();
        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let mut socket =
                accept_hdr_async(stream, move |request: &Request, response: Response| {
                    *captured.lock().unwrap() = request
                        .headers()
                        .get(AUTHORIZATION)
                        .unwrap()
                        .to_str()
                        .unwrap()
                        .to_owned();
                    Ok(response)
                })
                .await
                .unwrap();
            socket
                .send(Message::Text(
                    nextjson::json!({
                        "type":"session.created",
                        "session":{
                            "id":"session-1",
                            "model":"glm-realtime",
                            "modalities":["text","audio"],
                            "beta_fields":{"chat_mode":"audio"}
                        }
                    })
                    .to_string()
                    .into(),
                ))
                .await
                .unwrap();
            let mut received = Vec::new();
            while received.len() < 14 {
                if let Some(Ok(Message::Text(text))) = socket.next().await {
                    let event: Value = nextjson::from_str(&text).unwrap();
                    let kind = event["type"].as_str().unwrap().to_owned();
                    received.push(event);
                    if kind == "response.create" {
                        socket.send(Message::Ping(vec![1].into())).await.unwrap();
                        for event in [
                            nextjson::json!({"type":"response.text.delta","delta":"hello"}),
                            nextjson::json!({"type":"response.audio.delta","delta":"AQI="}),
                            nextjson::json!({"type":"response.done","response":{"status":"completed"}}),
                        ] {
                            socket
                                .send(Message::Text(event.to_string().into()))
                                .await
                                .unwrap();
                        }
                    }
                }
            }
            while let Some(message) = socket.next().await {
                if matches!(message.unwrap(), Message::Close(_)) {
                    break;
                }
            }
            received
        });

        let mut connection = RealtimeConfig::new("test-key")
            .authentication(ZhipuAuthentication::bearer("test-key"))
            .url(format!("ws://{address}"))
            .connect_timeout(Duration::from_secs(2))
            .channel_capacity(32)
            .connect()
            .await
            .unwrap();
        assert!(matches!(
            connection.next_typed_event().await.unwrap().unwrap(),
            RealtimeServerMessage::SessionCreated { .. }
        ));
        let sender = connection.sender();
        connection
            .send(&RealtimeClientEvent::session_update(RealtimeSession::default()).unwrap())
            .await
            .unwrap();
        sender
            .update_session(RealtimeSession::default())
            .await
            .unwrap();
        sender.append_audio(&[1, 2]).await.unwrap();
        sender.append_audio_base64("AQI=").await.unwrap();
        sender.append_video_frame(&[0xff, 0xd8]).await.unwrap();
        sender.commit().await.unwrap();
        sender.clear_audio().await.unwrap();
        sender
            .update_transcription_session(RealtimeTranscriptionSession::default())
            .await
            .unwrap();
        sender
            .create_item(RealtimeConversationItem::text("user", "hello"))
            .await
            .unwrap();
        sender.delete_item("item-1").await.unwrap();
        sender.retrieve_item("item-1").await.unwrap();
        sender.create_response().await.unwrap();
        sender.cancel_response().await.unwrap();
        sender
            .send_json(&nextjson::json!({"type":"custom.event"}))
            .await
            .unwrap();
        let mut text = String::new();
        let mut audio = Vec::new();
        while let Some(event) = connection.next_event().await {
            let event = event.unwrap();
            if let Some(delta) = event.delta_text() {
                text.push_str(delta);
            }
            if let Some(bytes) = event.audio_bytes().unwrap() {
                audio.extend(bytes);
            }
            if event.event_type == "response.done" {
                break;
            }
        }
        assert_eq!(text, "hello");
        assert_eq!(audio, vec![1, 2]);
        connection.close().await.unwrap();
        let received = server.await.unwrap();
        assert_eq!(authorization.lock().unwrap().as_str(), "Bearer test-key");
        assert_eq!(received[0]["type"].as_str(), Some("session.update"));
        assert_eq!(received[1]["type"].as_str(), Some("session.update"));
        assert_eq!(
            received[7]["type"].as_str(),
            Some("transcription_session.update")
        );
        assert_eq!(
            received[8]["type"].as_str(),
            Some("conversation.item.create")
        );
        assert_eq!(received[11]["type"].as_str(), Some("response.create"));
        assert_eq!(received[13]["type"].as_str(), Some("custom.event"));
    }

    #[tokio::test]
    async fn typed_sender_methods_enqueue_official_requests() {
        let (commands, mut receiver) = mpsc::channel(8);
        let sender = RealtimeSender { commands };

        sender
            .update_typed_session(TypedRealtimeSession::default())
            .await
            .unwrap();
        sender
            .create_typed_item(None, TypedRealtimeItem::function_output("call-1", "ok"))
            .await
            .unwrap();
        sender
            .create_response_with(RealtimeResponseOptions::default())
            .await
            .unwrap();
        sender
            .send_request(&RealtimeRequest::cancel_response().unwrap())
            .await
            .unwrap();

        let mut kinds = Vec::new();
        for _ in 0..4 {
            let RealtimeCommand::Send(Message::Text(text)) = receiver.recv().await.unwrap() else {
                panic!("expected text command");
            };
            let value: Value = nextjson::from_str(&text).unwrap();
            kinds.push(value["type"].as_str().unwrap().to_owned());
        }
        assert_eq!(
            kinds,
            [
                "session.update",
                "conversation.item.create",
                "response.create",
                "response.cancel"
            ]
        );
    }
}
