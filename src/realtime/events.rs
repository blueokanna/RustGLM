use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::{Map, Value};

use super::{
    RealtimeEventMetadata, RealtimeMaxTokens, RealtimeServerEvent, RealtimeTranscriptionSession,
};
use crate::{Result, SdkError};

macro_rules! string_enum {
    ($(#[$meta:meta])* pub enum $name:ident { $($variant:ident => $value:literal),+ $(,)? }) => {
        $(#[$meta])*
        #[derive(Debug, Clone, PartialEq, Eq, Hash)]
        pub enum $name {
            $($variant,)+
            Other(String),
        }

        impl $name {
            pub fn as_str(&self) -> &str {
                match self {
                    $(Self::$variant => $value,)+
                    Self::Other(value) => value,
                }
            }
        }

        impl From<&str> for $name {
            fn from(value: &str) -> Self {
                match value {
                    $($value => Self::$variant,)+
                    value => Self::Other(value.to_owned()),
                }
            }
        }

        impl From<String> for $name {
            fn from(value: String) -> Self {
                Self::from(value.as_str())
            }
        }

        impl Serialize for $name {
            fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
            where
                S: Serializer,
            {
                serializer.serialize_str(self.as_str())
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
            where
                D: Deserializer<'de>,
            {
                String::deserialize(deserializer).map(Self::from)
            }
        }
    };
}

string_enum! {
    /// Model identifiers documented by the current Realtime API.
    pub enum RealtimeModel {
        GlmRealtime => "glm-realtime",
        GlmRealtimeFlash => "glm-realtime-flash",
        GlmRealtimeAir => "glm-realtime-air"
    }
}

#[allow(clippy::derivable_impls)]
impl Default for RealtimeModel {
    fn default() -> Self {
        Self::GlmRealtime
    }
}

string_enum! {
    pub enum RealtimeModality {
        Text => "text",
        Audio => "audio"
    }
}

string_enum! {
    pub enum RealtimeAudioFormat {
        Wav => "wav",
        Mp3 => "mp3",
        Pcm => "pcm",
        Pcm16 => "pcm16",
        Pcm24 => "pcm24"
    }
}

string_enum! {
    pub enum RealtimeChatMode {
        Audio => "audio",
        VideoPassive => "video_passive",
        VideoProactive => "video_proactive"
    }
}

string_enum! {
    pub enum RealtimeTtsSource {
        Zhipu => "zhipu",
        Huoshan => "huoshan",
        E2e => "e2e"
    }
}

string_enum! {
    pub enum RealtimeRole {
        System => "system",
        User => "user",
        Assistant => "assistant"
    }
}

string_enum! {
    pub enum RealtimeItemStatus {
        InProgress => "in_progress",
        Completed => "completed",
        Incomplete => "incomplete"
    }
}

string_enum! {
    pub enum RealtimeResponseStatus {
        InProgress => "in_progress",
        Completed => "completed",
        Cancelled => "cancelled",
        Incomplete => "incomplete",
        Failed => "failed"
    }
}

string_enum! {
    pub enum RealtimeToolChoiceMode {
        Auto => "auto",
        None => "none",
        Required => "required"
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum TypedRealtimeTurnDetection {
    #[serde(rename = "none")]
    None,
    #[serde(rename = "client_vad")]
    ClientVad,
    #[serde(rename = "server_vad")]
    ServerVad {
        #[serde(skip_serializing_if = "Option::is_none")]
        create_response: Option<bool>,
        #[serde(skip_serializing_if = "Option::is_none")]
        interrupt_response: Option<bool>,
        #[serde(skip_serializing_if = "Option::is_none")]
        threshold: Option<f32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        prefix_padding_ms: Option<u32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        silence_duration_ms: Option<u32>,
    },
}

impl TypedRealtimeTurnDetection {
    pub fn server_vad() -> Self {
        Self::ServerVad {
            create_response: None,
            interrupt_response: None,
            threshold: None,
            prefix_padding_ms: None,
            silence_duration_ms: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TypedRealtimeTool {
    #[serde(rename = "type")]
    kind: RealtimeFunctionToolKind,
    pub name: String,
    pub description: String,
    pub parameters: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum RealtimeFunctionToolKind {
    Function,
}

impl TypedRealtimeTool {
    pub fn function(
        name: impl Into<String>,
        description: impl Into<String>,
        parameters: Value,
    ) -> Self {
        Self {
            kind: RealtimeFunctionToolKind::Function,
            name: name.into(),
            description: description.into(),
            parameters,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum RealtimeToolChoice {
    Mode(RealtimeToolChoiceMode),
    Function {
        #[serde(rename = "type")]
        kind: RealtimeFunctionToolKind,
        function: String,
    },
}

impl RealtimeToolChoice {
    pub fn function(name: impl Into<String>) -> Self {
        Self::Function {
            kind: RealtimeFunctionToolKind::Function,
            function: name.into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TypedRealtimeBetaFields {
    pub chat_mode: RealtimeChatMode,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tts_source: Option<RealtimeTtsSource>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_search: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_size_x: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_size_y: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fps: Option<u32>,
}

impl Default for TypedRealtimeBetaFields {
    fn default() -> Self {
        Self {
            chat_mode: RealtimeChatMode::Audio,
            tts_source: None,
            auto_search: None,
            image_size_x: None,
            image_size_y: None,
            fps: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TypedRealtimeSession {
    pub model: RealtimeModel,
    pub modalities: Vec<RealtimeModality>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instructions: Option<String>,
    pub voice: String,
    pub input_audio_format: RealtimeAudioFormat,
    pub output_audio_format: RealtimeAudioFormat,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_audio_transcription: Option<RealtimeInputAudioTranscription>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub turn_detection: Option<TypedRealtimeTurnDetection>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tools: Vec<TypedRealtimeTool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<RealtimeToolChoice>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_output_tokens: Option<RealtimeMaxTokens>,
    pub beta_fields: TypedRealtimeBetaFields,
    #[serde(flatten, default, skip_serializing_if = "Map::is_empty")]
    pub extra: Map<String, Value>,
}

impl Default for TypedRealtimeSession {
    fn default() -> Self {
        Self {
            model: RealtimeModel::default(),
            modalities: vec![RealtimeModality::Text, RealtimeModality::Audio],
            instructions: None,
            voice: "tongtong".into(),
            input_audio_format: RealtimeAudioFormat::Pcm16,
            output_audio_format: RealtimeAudioFormat::Pcm,
            input_audio_transcription: None,
            turn_detection: None,
            tools: Vec::new(),
            tool_choice: None,
            temperature: None,
            max_output_tokens: None,
            beta_fields: TypedRealtimeBetaFields::default(),
            extra: Map::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RealtimeInputAudioTranscription {
    pub model: RealtimeTranscriptionModel,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum RealtimeTranscriptionModel {
    #[serde(rename = "whisper-1")]
    Whisper1,
}

impl TypedRealtimeSession {
    pub fn model(mut self, value: RealtimeModel) -> Self {
        self.model = value;
        self
    }

    pub fn instructions(mut self, value: impl Into<String>) -> Self {
        self.instructions = Some(value.into());
        self
    }

    pub fn server_vad(mut self) -> Self {
        self.turn_detection = Some(TypedRealtimeTurnDetection::server_vad());
        self
    }

    pub fn video(mut self) -> Self {
        self.beta_fields.chat_mode = RealtimeChatMode::VideoPassive;
        self
    }

    pub fn tool(mut self, value: TypedRealtimeTool) -> Self {
        self.tools.push(value);
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum TypedRealtimeContentPart {
    #[serde(rename = "input_text")]
    InputText { text: String },
    #[serde(rename = "input_audio")]
    InputAudio {
        audio: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        transcript: Option<String>,
    },
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "audio")]
    Audio {
        #[serde(skip_serializing_if = "Option::is_none")]
        transcript: Option<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum TypedRealtimeItem {
    #[serde(rename = "message")]
    Message {
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        role: RealtimeRole,
        content: Vec<TypedRealtimeContentPart>,
        #[serde(skip_serializing_if = "Option::is_none")]
        status: Option<RealtimeItemStatus>,
    },
    #[serde(rename = "function_call")]
    FunctionCall {
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        name: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        call_id: Option<String>,
        arguments: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        status: Option<RealtimeItemStatus>,
    },
    #[serde(rename = "function_call_output")]
    FunctionCallOutput {
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        call_id: String,
        output: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        status: Option<RealtimeItemStatus>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum RealtimeResponseContentPart {
    #[serde(rename = "input_text")]
    InputText { text: String },
    #[serde(rename = "input_audio")]
    InputAudio {
        #[serde(default)]
        transcript: Option<String>,
    },
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "audio")]
    Audio {
        #[serde(default)]
        transcript: Option<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum RealtimeResponseItem {
    #[serde(rename = "message")]
    Message {
        #[serde(default)]
        id: Option<String>,
        #[serde(default)]
        status: Option<RealtimeItemStatus>,
        #[serde(default)]
        role: Option<RealtimeRole>,
        #[serde(default)]
        content: Vec<RealtimeResponseContentPart>,
    },
    #[serde(rename = "function_call")]
    FunctionCall {
        #[serde(default)]
        id: Option<String>,
        #[serde(default)]
        status: Option<RealtimeItemStatus>,
        #[serde(default)]
        name: String,
        #[serde(default)]
        call_id: Option<String>,
        #[serde(default)]
        arguments: String,
    },
    #[serde(rename = "function_call_output")]
    FunctionCallOutput {
        #[serde(default)]
        id: Option<String>,
        #[serde(default)]
        call_id: Option<String>,
        #[serde(default)]
        output: Option<String>,
    },
}

impl TypedRealtimeItem {
    pub fn user_text(text: impl Into<String>) -> Self {
        Self::Message {
            id: None,
            role: RealtimeRole::User,
            content: vec![TypedRealtimeContentPart::InputText { text: text.into() }],
            status: Some(RealtimeItemStatus::Completed),
        }
    }

    pub fn function_output(call_id: impl Into<String>, output: impl Into<String>) -> Self {
        Self::FunctionCallOutput {
            id: None,
            call_id: call_id.into(),
            output: output.into(),
            status: Some(RealtimeItemStatus::Completed),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RealtimeResponseOptions {
    pub commit: bool,
    pub cancel_previous: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub append_input_items: Vec<TypedRealtimeItem>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub input_items: Vec<TypedRealtimeItem>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instructions: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub modalities: Vec<RealtimeModality>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub voice: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_output_tokens: Option<RealtimeMaxTokens>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tools: Vec<TypedRealtimeTool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<RealtimeToolChoice>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_audio_format: Option<RealtimeAudioFormat>,
}

impl Default for RealtimeResponseOptions {
    fn default() -> Self {
        Self {
            commit: true,
            cancel_previous: true,
            append_input_items: Vec::new(),
            input_items: Vec::new(),
            instructions: None,
            modalities: Vec::new(),
            voice: None,
            temperature: None,
            max_output_tokens: None,
            tools: Vec::new(),
            tool_choice: None,
            output_audio_format: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum RealtimeRequest {
    #[serde(rename = "session.update")]
    SessionUpdate {
        #[serde(flatten)]
        metadata: RealtimeEventMetadata,
        session: Box<TypedRealtimeSession>,
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
        #[serde(skip_serializing_if = "Option::is_none")]
        previous_item_id: Option<String>,
        item: Box<TypedRealtimeItem>,
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
        #[serde(skip_serializing_if = "Option::is_none")]
        response: Option<Box<RealtimeResponseOptions>>,
    },
    #[serde(rename = "response.cancel")]
    ResponseCancel {
        #[serde(flatten)]
        metadata: RealtimeEventMetadata,
    },
}

impl RealtimeRequest {
    pub fn session_update(session: TypedRealtimeSession) -> Result<Self> {
        Ok(Self::SessionUpdate {
            metadata: RealtimeEventMetadata::new()?,
            session: Box::new(session),
        })
    }

    pub fn transcription_session_update(session: RealtimeTranscriptionSession) -> Result<Self> {
        Ok(Self::TranscriptionSessionUpdate {
            metadata: RealtimeEventMetadata::new()?,
            session: Box::new(session),
        })
    }

    pub fn append_audio(bytes: &[u8]) -> Result<Self> {
        if bytes.is_empty() {
            return Err(SdkError::Validation("audio data cannot be empty".into()));
        }
        Ok(Self::InputAudioBufferAppend {
            metadata: RealtimeEventMetadata::new()?,
            audio: STANDARD.encode(bytes),
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

    pub fn create_item(previous_item_id: Option<String>, item: TypedRealtimeItem) -> Result<Self> {
        Ok(Self::ConversationItemCreate {
            metadata: RealtimeEventMetadata::new()?,
            previous_item_id,
            item: Box::new(item),
        })
    }

    pub fn delete_item(item_id: impl Into<String>) -> Result<Self> {
        let item_id = require_non_empty(item_id.into(), "item id")?;
        Ok(Self::ConversationItemDelete {
            metadata: RealtimeEventMetadata::new()?,
            item_id,
        })
    }

    pub fn retrieve_item(item_id: impl Into<String>) -> Result<Self> {
        let item_id = require_non_empty(item_id.into(), "item id")?;
        Ok(Self::ConversationItemRetrieve {
            metadata: RealtimeEventMetadata::new()?,
            item_id,
        })
    }

    pub fn create_response(options: Option<RealtimeResponseOptions>) -> Result<Self> {
        Ok(Self::ResponseCreate {
            metadata: RealtimeEventMetadata::new()?,
            response: options.map(Box::new),
        })
    }

    pub fn cancel_response() -> Result<Self> {
        Ok(Self::ResponseCancel {
            metadata: RealtimeEventMetadata::new()?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RealtimeServerMetadata {
    pub event_id: Option<String>,
    pub client_timestamp: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct RealtimeSessionState {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub model: Option<RealtimeModel>,
    #[serde(default)]
    pub modalities: Vec<RealtimeModality>,
    #[serde(default)]
    pub instructions: Option<String>,
    #[serde(default)]
    pub voice: Option<String>,
    #[serde(default)]
    pub input_audio_format: Option<RealtimeAudioFormat>,
    #[serde(default)]
    pub output_audio_format: Option<RealtimeAudioFormat>,
    #[serde(default)]
    pub input_audio_transcription: Option<RealtimeInputAudioTranscription>,
    #[serde(default)]
    pub turn_detection: Option<TypedRealtimeTurnDetection>,
    #[serde(default)]
    pub tools: Option<Vec<TypedRealtimeTool>>,
    #[serde(default)]
    pub tool_choice: Option<RealtimeToolChoice>,
    #[serde(default)]
    pub temperature: Option<f32>,
    #[serde(default)]
    pub beta_fields: Option<TypedRealtimeBetaFields>,
    #[serde(flatten, default)]
    pub extra: Map<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct RealtimeErrorDetail {
    #[serde(default)]
    pub message: String,
    #[serde(default)]
    #[serde(rename = "type")]
    pub kind: Option<String>,
    #[serde(default)]
    pub code: Option<String>,
    #[serde(default)]
    pub param: Option<String>,
    #[serde(default)]
    pub event_id: Option<String>,
    #[serde(flatten, default)]
    pub extra: Map<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct RealtimeTokenDetails {
    #[serde(default)]
    pub cached_tokens: Option<u64>,
    #[serde(default)]
    pub text_tokens: Option<u64>,
    #[serde(default)]
    pub audio_tokens: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct RealtimeUsage {
    #[serde(default)]
    pub total_tokens: Option<u64>,
    #[serde(default)]
    pub input_tokens: Option<u64>,
    #[serde(default)]
    pub output_tokens: Option<u64>,
    #[serde(default)]
    pub input_token_details: Option<RealtimeTokenDetails>,
    #[serde(default)]
    pub output_token_details: Option<RealtimeTokenDetails>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RealtimeResponse {
    pub id: String,
    #[serde(default)]
    pub object: Option<String>,
    pub status: RealtimeResponseStatus,
    #[serde(default)]
    pub status_details: Option<Value>,
    #[serde(default)]
    pub output: Option<Vec<RealtimeResponseItem>>,
    #[serde(default)]
    pub usage: Option<RealtimeUsage>,
    #[serde(flatten, default)]
    pub extra: Map<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct RealtimeDelta {
    #[serde(default)]
    pub response_id: Option<String>,
    #[serde(default)]
    pub item_id: Option<String>,
    #[serde(default)]
    pub output_index: Option<u32>,
    #[serde(default)]
    pub content_index: Option<u32>,
    #[serde(default)]
    pub delta: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct RealtimeFunctionCallDone {
    #[serde(default)]
    pub response_id: Option<String>,
    #[serde(default)]
    pub item_id: Option<String>,
    #[serde(default)]
    pub output_index: Option<u32>,
    #[serde(default)]
    pub call_id: Option<String>,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub arguments: String,
}

impl RealtimeFunctionCallDone {
    pub fn arguments<T: DeserializeOwned>(&self) -> serde_json::Result<T> {
        serde_json::from_str(&self.arguments)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum RealtimeServerMessage {
    Error {
        metadata: RealtimeServerMetadata,
        error: RealtimeErrorDetail,
    },
    Heartbeat {
        metadata: RealtimeServerMetadata,
    },
    SessionCreated {
        metadata: RealtimeServerMetadata,
        session: RealtimeSessionState,
    },
    SessionUpdated {
        metadata: RealtimeServerMetadata,
        session: RealtimeSessionState,
    },
    AudioBufferCommitted {
        metadata: RealtimeServerMetadata,
        previous_item_id: Option<String>,
        item_id: Option<String>,
    },
    SpeechStarted {
        metadata: RealtimeServerMetadata,
        audio_start_ms: Option<u64>,
        item_id: Option<String>,
    },
    SpeechStopped {
        metadata: RealtimeServerMetadata,
        audio_end_ms: Option<u64>,
        item_id: Option<String>,
    },
    AudioBufferCleared {
        metadata: RealtimeServerMetadata,
    },
    ConversationCreated {
        metadata: RealtimeServerMetadata,
        conversation: Value,
    },
    ItemCreated {
        metadata: RealtimeServerMetadata,
        previous_item_id: Option<String>,
        item: RealtimeResponseItem,
    },
    TranscriptionCompleted {
        metadata: RealtimeServerMetadata,
        item_id: Option<String>,
        content_index: Option<u32>,
        transcript: Option<String>,
    },
    TranscriptionFailed {
        metadata: RealtimeServerMetadata,
        item_id: Option<String>,
        content_index: Option<u32>,
        error: Option<RealtimeErrorDetail>,
    },
    ResponseCreated {
        metadata: RealtimeServerMetadata,
        response: RealtimeResponse,
    },
    ResponseDone {
        metadata: RealtimeServerMetadata,
        response: RealtimeResponse,
    },
    TextDelta {
        metadata: RealtimeServerMetadata,
        delta: RealtimeDelta,
    },
    AudioTranscriptDelta {
        metadata: RealtimeServerMetadata,
        delta: RealtimeDelta,
    },
    AudioTranscriptDone {
        metadata: RealtimeServerMetadata,
        response_id: Option<String>,
        item_id: Option<String>,
        output_index: Option<u32>,
        content_index: Option<u32>,
        transcript: Option<String>,
    },
    AudioDelta {
        metadata: RealtimeServerMetadata,
        delta: RealtimeDelta,
    },
    AudioDone {
        metadata: RealtimeServerMetadata,
        response_id: Option<String>,
        item_id: Option<String>,
        output_index: Option<u32>,
        content_index: Option<u32>,
    },
    FunctionCallArgumentsDone {
        metadata: RealtimeServerMetadata,
        call: RealtimeFunctionCallDone,
    },
    Unknown(RealtimeServerEvent),
}

impl RealtimeServerMessage {
    pub fn event_type(&self) -> &str {
        match self {
            Self::Error { .. } => "error",
            Self::Heartbeat { .. } => "heartbeat",
            Self::SessionCreated { .. } => "session.created",
            Self::SessionUpdated { .. } => "session.updated",
            Self::AudioBufferCommitted { .. } => "input_audio_buffer.committed",
            Self::SpeechStarted { .. } => "input_audio_buffer.speech_started",
            Self::SpeechStopped { .. } => "input_audio_buffer.speech_stopped",
            Self::AudioBufferCleared { .. } => "input_audio_buffer.cleared",
            Self::ConversationCreated { .. } => "conversation.created",
            Self::ItemCreated { .. } => "conversation.item.created",
            Self::TranscriptionCompleted { .. } => {
                "conversation.item.input_audio_transcription.completed"
            }
            Self::TranscriptionFailed { .. } => {
                "conversation.item.input_audio_transcription.failed"
            }
            Self::ResponseCreated { .. } => "response.created",
            Self::ResponseDone { .. } => "response.done",
            Self::TextDelta { .. } => "response.text.delta",
            Self::AudioTranscriptDelta { .. } => "response.audio_transcript.delta",
            Self::AudioTranscriptDone { .. } => "response.audio_transcript.done",
            Self::AudioDelta { .. } => "response.audio.delta",
            Self::AudioDone { .. } => "response.audio.done",
            Self::FunctionCallArgumentsDone { .. } => "response.function_call_arguments.done",
            Self::Unknown(event) => &event.event_type,
        }
    }

    pub fn delta_text(&self) -> Option<&str> {
        match self {
            Self::TextDelta { delta, .. } | Self::AudioTranscriptDelta { delta, .. } => {
                delta.delta.as_deref()
            }
            _ => None,
        }
    }

    pub fn audio_bytes(&self) -> Result<Option<Vec<u8>>> {
        let Some(value) = (match self {
            Self::AudioDelta { delta, .. } => delta.delta.as_deref(),
            _ => None,
        }) else {
            return Ok(None);
        };
        STANDARD
            .decode(value)
            .map(Some)
            .map_err(|error| SdkError::Stream(error.to_string().into()))
    }

    pub fn function_call(&self) -> Option<&RealtimeFunctionCallDone> {
        match self {
            Self::FunctionCallArgumentsDone { call, .. } => Some(call),
            _ => None,
        }
    }

    pub fn error(&self) -> Option<&RealtimeErrorDetail> {
        match self {
            Self::Error { error, .. } => Some(error),
            _ => None,
        }
    }
}

impl RealtimeServerEvent {
    /// Converts a raw event to the current strongly typed protocol model.
    ///
    /// Unknown future events and known events with a newer payload shape are retained verbatim in
    /// [`RealtimeServerMessage::Unknown`].
    pub fn into_typed(self) -> RealtimeServerMessage {
        let metadata = RealtimeServerMetadata {
            event_id: self.event_id.clone(),
            client_timestamp: self.client_timestamp,
        };
        macro_rules! decode {
            ($payload:ty, $body:expr) => {
                match decode_payload::<$payload>(&self.data) {
                    Ok(payload) => $body(metadata, payload),
                    Err(_) => RealtimeServerMessage::Unknown(self),
                }
            };
        }

        match self.event_type.as_str() {
            "error" => decode!(ErrorPayload, |metadata, payload: ErrorPayload| {
                RealtimeServerMessage::Error {
                    metadata,
                    error: payload.error,
                }
            }),
            "heartbeat" => RealtimeServerMessage::Heartbeat { metadata },
            "session.created" => decode!(SessionPayload, |metadata, payload: SessionPayload| {
                RealtimeServerMessage::SessionCreated {
                    metadata,
                    session: payload.session,
                }
            }),
            "session.updated" => decode!(SessionPayload, |metadata, payload: SessionPayload| {
                RealtimeServerMessage::SessionUpdated {
                    metadata,
                    session: payload.session,
                }
            }),
            "input_audio_buffer.committed" => {
                decode!(
                    BufferCommittedPayload,
                    |metadata, payload: BufferCommittedPayload| {
                        RealtimeServerMessage::AudioBufferCommitted {
                            metadata,
                            previous_item_id: payload.previous_item_id,
                            item_id: payload.item_id,
                        }
                    }
                )
            }
            "input_audio_buffer.speech_started" => {
                decode!(
                    SpeechStartedPayload,
                    |metadata, payload: SpeechStartedPayload| {
                        RealtimeServerMessage::SpeechStarted {
                            metadata,
                            audio_start_ms: payload.audio_start_ms,
                            item_id: payload.item_id,
                        }
                    }
                )
            }
            "input_audio_buffer.speech_stopped" => {
                decode!(
                    SpeechStoppedPayload,
                    |metadata, payload: SpeechStoppedPayload| {
                        RealtimeServerMessage::SpeechStopped {
                            metadata,
                            audio_end_ms: payload.audio_end_ms,
                            item_id: payload.item_id,
                        }
                    }
                )
            }
            "input_audio_buffer.cleared" => RealtimeServerMessage::AudioBufferCleared { metadata },
            "conversation.created" => decode!(
                ConversationPayload,
                |metadata, payload: ConversationPayload| {
                    RealtimeServerMessage::ConversationCreated {
                        metadata,
                        conversation: payload.conversation,
                    }
                }
            ),
            "conversation.item.created" => decode!(
                ItemCreatedPayload,
                |metadata, payload: ItemCreatedPayload| {
                    RealtimeServerMessage::ItemCreated {
                        metadata,
                        previous_item_id: payload.previous_item_id,
                        item: payload.item,
                    }
                }
            ),
            "conversation.item.input_audio_transcription.completed" => {
                decode!(
                    TranscriptionCompletedPayload,
                    |metadata, payload: TranscriptionCompletedPayload| {
                        RealtimeServerMessage::TranscriptionCompleted {
                            metadata,
                            item_id: payload.item_id,
                            content_index: payload.content_index,
                            transcript: payload.transcript,
                        }
                    }
                )
            }
            "conversation.item.input_audio_transcription.failed" => {
                decode!(
                    TranscriptionFailedPayload,
                    |metadata, payload: TranscriptionFailedPayload| {
                        RealtimeServerMessage::TranscriptionFailed {
                            metadata,
                            item_id: payload.item_id,
                            content_index: payload.content_index,
                            error: payload.error,
                        }
                    }
                )
            }
            "response.created" => decode!(ResponsePayload, |metadata, payload: ResponsePayload| {
                RealtimeServerMessage::ResponseCreated {
                    metadata,
                    response: payload.response,
                }
            }),
            "response.done" => decode!(ResponsePayload, |metadata, payload: ResponsePayload| {
                RealtimeServerMessage::ResponseDone {
                    metadata,
                    response: payload.response,
                }
            }),
            "response.text.delta" => decode!(RealtimeDelta, |metadata, delta: RealtimeDelta| {
                RealtimeServerMessage::TextDelta { metadata, delta }
            }),
            "response.audio_transcript.delta" => {
                decode!(RealtimeDelta, |metadata, delta: RealtimeDelta| {
                    RealtimeServerMessage::AudioTranscriptDelta { metadata, delta }
                })
            }
            "response.audio_transcript.done" => {
                decode!(
                    TranscriptDonePayload,
                    |metadata, payload: TranscriptDonePayload| {
                        RealtimeServerMessage::AudioTranscriptDone {
                            metadata,
                            response_id: payload.response_id,
                            item_id: payload.item_id,
                            output_index: payload.output_index,
                            content_index: payload.content_index,
                            transcript: payload.transcript,
                        }
                    }
                )
            }
            "response.audio.delta" => decode!(RealtimeDelta, |metadata, delta: RealtimeDelta| {
                RealtimeServerMessage::AudioDelta { metadata, delta }
            }),
            "response.audio.done" => {
                decode!(AudioDonePayload, |metadata, payload: AudioDonePayload| {
                    RealtimeServerMessage::AudioDone {
                        metadata,
                        response_id: payload.response_id,
                        item_id: payload.item_id,
                        output_index: payload.output_index,
                        content_index: payload.content_index,
                    }
                })
            }
            "response.function_call_arguments.done" => {
                decode!(
                    RealtimeFunctionCallDone,
                    |metadata, call: RealtimeFunctionCallDone| {
                        RealtimeServerMessage::FunctionCallArgumentsDone { metadata, call }
                    }
                )
            }
            _ => RealtimeServerMessage::Unknown(self),
        }
    }
}

fn decode_payload<T: DeserializeOwned>(data: &Map<String, Value>) -> serde_json::Result<T> {
    serde_json::from_value(Value::Object(data.clone()))
}

fn require_non_empty(value: String, name: &str) -> Result<String> {
    if value.trim().is_empty() {
        return Err(SdkError::Validation(
            format!("{name} cannot be empty").into(),
        ));
    }
    Ok(value)
}

#[derive(Deserialize)]
struct ErrorPayload {
    error: RealtimeErrorDetail,
}

#[derive(Deserialize)]
struct SessionPayload {
    session: RealtimeSessionState,
}

#[derive(Deserialize)]
struct BufferCommittedPayload {
    #[serde(default)]
    previous_item_id: Option<String>,
    #[serde(default)]
    item_id: Option<String>,
}

#[derive(Deserialize)]
struct SpeechStartedPayload {
    #[serde(default)]
    audio_start_ms: Option<u64>,
    #[serde(default)]
    item_id: Option<String>,
}

#[derive(Deserialize)]
struct SpeechStoppedPayload {
    #[serde(default)]
    audio_end_ms: Option<u64>,
    #[serde(default)]
    item_id: Option<String>,
}

#[derive(Deserialize)]
struct ConversationPayload {
    conversation: Value,
}

#[derive(Deserialize)]
struct ItemCreatedPayload {
    #[serde(default)]
    previous_item_id: Option<String>,
    item: RealtimeResponseItem,
}

#[derive(Deserialize)]
struct TranscriptionCompletedPayload {
    #[serde(default)]
    item_id: Option<String>,
    #[serde(default)]
    content_index: Option<u32>,
    #[serde(default)]
    transcript: Option<String>,
}

#[derive(Deserialize)]
struct TranscriptionFailedPayload {
    #[serde(default)]
    item_id: Option<String>,
    #[serde(default)]
    content_index: Option<u32>,
    #[serde(default)]
    error: Option<RealtimeErrorDetail>,
}

#[derive(Deserialize)]
struct ResponsePayload {
    response: RealtimeResponse,
}

#[derive(Deserialize)]
struct TranscriptDonePayload {
    #[serde(default)]
    response_id: Option<String>,
    #[serde(default)]
    item_id: Option<String>,
    #[serde(default)]
    output_index: Option<u32>,
    #[serde(default)]
    content_index: Option<u32>,
    #[serde(default)]
    transcript: Option<String>,
}

#[derive(Deserialize)]
struct AudioDonePayload {
    #[serde(default)]
    response_id: Option<String>,
    #[serde(default)]
    item_id: Option<String>,
    #[serde(default)]
    output_index: Option<u32>,
    #[serde(default)]
    content_index: Option<u32>,
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;
    use serde_json::json;

    use super::*;

    #[test]
    fn typed_request_serializes_session_items_and_response_options() {
        let session = TypedRealtimeSession::default()
            .model(RealtimeModel::GlmRealtimeFlash)
            .server_vad()
            .video()
            .tool(TypedRealtimeTool::function(
                "weather",
                "Get weather",
                json!({"type":"object"}),
            ));
        let value =
            serde_json::to_value(RealtimeRequest::session_update(session).unwrap()).unwrap();
        assert_eq!(value["type"], "session.update");
        assert_eq!(value["session"]["model"], "glm-realtime-flash");
        assert_eq!(value["session"]["turn_detection"]["type"], "server_vad");
        assert_eq!(
            value["session"]["beta_fields"]["chat_mode"],
            "video_passive"
        );

        let item = TypedRealtimeItem::function_output("call-1", r#"{"ok":true}"#);
        let request = RealtimeRequest::create_item(None, item).unwrap();
        let value = serde_json::to_value(request).unwrap();
        assert_eq!(value["item"]["call_id"], "call-1");

        let options = RealtimeResponseOptions {
            instructions: Some("brief".into()),
            ..RealtimeResponseOptions::default()
        };
        let value =
            serde_json::to_value(RealtimeRequest::create_response(Some(options)).unwrap()).unwrap();
        assert_eq!(value["response"]["commit"], true);
        assert_eq!(value["response"]["cancel_previous"], true);
    }

    #[test]
    fn decodes_typed_server_messages_and_retains_unknown_events() {
        let raw: RealtimeServerEvent = serde_json::from_value(json!({
            "type":"response.function_call_arguments.done",
            "event_id":"event-1",
            "response_id":"response-1",
            "call_id":"call-1",
            "name":"weather",
            "arguments":"{\"city\":\"Beijing\"}"
        }))
        .unwrap();
        let event = raw.into_typed();
        let call = event.function_call().unwrap();
        #[derive(Deserialize)]
        struct Arguments {
            city: String,
        }
        assert_eq!(call.arguments::<Arguments>().unwrap().city, "Beijing");

        let raw: RealtimeServerEvent = serde_json::from_value(json!({
            "type":"response.future.delta",
            "delta":"kept"
        }))
        .unwrap();
        assert!(matches!(
            raw.into_typed(),
            RealtimeServerMessage::Unknown(_)
        ));
    }

    #[test]
    fn decodes_audio_and_complete_response_usage() {
        let audio: RealtimeServerEvent = serde_json::from_value(json!({
            "type":"response.audio.delta",
            "delta":"AQI="
        }))
        .unwrap();
        assert_eq!(audio.into_typed().audio_bytes().unwrap(), Some(vec![1, 2]));

        let done: RealtimeServerEvent = serde_json::from_value(json!({
            "type":"response.done",
            "response": {
                "id":"response-1",
                "object":"realtime.response",
                "status":"completed",
                "output":[],
                "usage":{"total_tokens":12,"input_tokens":7,"output_tokens":5}
            }
        }))
        .unwrap();
        match done.into_typed() {
            RealtimeServerMessage::ResponseDone { response, .. } => {
                assert_eq!(response.usage.unwrap().total_tokens, Some(12));
            }
            event => panic!("unexpected event: {event:?}"),
        }
    }

    #[test]
    fn covers_every_documented_server_event_family() {
        let session = json!({
            "id":"session-1",
            "model":"glm-realtime-air",
            "modalities":["text","audio"],
            "beta_fields":{"chat_mode":"audio"}
        });
        let response = json!({
            "id":"response-1",
            "object":"realtime.response",
            "status":"in_progress",
            "output":[]
        });
        let cases = vec![
            (
                json!({"type":"error","error":{"message":"bad","type":"request_error"}}),
                "error",
            ),
            (json!({"type":"heartbeat"}), "heartbeat"),
            (
                json!({"type":"session.created","session":session.clone()}),
                "session.created",
            ),
            (
                json!({"type":"session.updated","session":session}),
                "session.updated",
            ),
            (
                json!({"type":"input_audio_buffer.committed","previous_item_id":"old","item_id":"new"}),
                "input_audio_buffer.committed",
            ),
            (
                json!({"type":"input_audio_buffer.speech_started","audio_start_ms":10,"item_id":"item"}),
                "input_audio_buffer.speech_started",
            ),
            (
                json!({"type":"input_audio_buffer.speech_stopped","audio_end_ms":20,"item_id":"item"}),
                "input_audio_buffer.speech_stopped",
            ),
            (
                json!({"type":"input_audio_buffer.cleared"}),
                "input_audio_buffer.cleared",
            ),
            (
                json!({"type":"conversation.created","conversation":{"id":"conversation-1"}}),
                "conversation.created",
            ),
            (
                json!({
                    "type":"conversation.item.created",
                    "previous_item_id":"previous",
                    "item":{"type":"message","id":"item","role":"user","content":[{"type":"input_text","text":"hello"}]}
                }),
                "conversation.item.created",
            ),
            (
                json!({
                    "type":"conversation.item.input_audio_transcription.completed",
                    "item_id":"item","content_index":0,"transcript":"hello"
                }),
                "conversation.item.input_audio_transcription.completed",
            ),
            (
                json!({
                    "type":"conversation.item.input_audio_transcription.failed",
                    "item_id":"item","content_index":0,"error":{"message":"bad audio"}
                }),
                "conversation.item.input_audio_transcription.failed",
            ),
            (
                json!({"type":"response.created","response":response}),
                "response.created",
            ),
            (
                json!({"type":"response.text.delta","delta":"text"}),
                "response.text.delta",
            ),
            (
                json!({"type":"response.audio_transcript.delta","delta":"words"}),
                "response.audio_transcript.delta",
            ),
            (
                json!({
                    "type":"response.audio_transcript.done","response_id":"response-1",
                    "item_id":"item","output_index":0,"content_index":0,"transcript":"done"
                }),
                "response.audio_transcript.done",
            ),
            (
                json!({
                    "type":"response.audio.done","response_id":"response-1",
                    "item_id":"item","output_index":0,"content_index":0
                }),
                "response.audio.done",
            ),
        ];

        for (value, expected) in cases {
            let raw: RealtimeServerEvent = serde_json::from_value(value).unwrap();
            let event = raw.into_typed();
            assert_eq!(event.event_type(), expected);
            assert!(!matches!(event, RealtimeServerMessage::Unknown(_)));
        }

        let error: RealtimeServerEvent = serde_json::from_value(json!({
            "type":"error","error":{"message":"bad","type":"request_error"}
        }))
        .unwrap();
        assert_eq!(
            error.into_typed().error().unwrap().kind.as_deref(),
            Some("request_error")
        );

        let text: RealtimeServerEvent = serde_json::from_value(json!({
            "type":"response.text.delta","delta":"hello"
        }))
        .unwrap();
        assert_eq!(text.into_typed().delta_text(), Some("hello"));
    }

    #[test]
    fn all_typed_client_request_constructors_validate_and_serialize() {
        let requests = vec![
            RealtimeRequest::transcription_session_update(RealtimeTranscriptionSession::default())
                .unwrap(),
            RealtimeRequest::append_audio(&[1, 2]).unwrap(),
            RealtimeRequest::append_video_frame(&[0xff, 0xd8]).unwrap(),
            RealtimeRequest::commit().unwrap(),
            RealtimeRequest::clear().unwrap(),
            RealtimeRequest::delete_item("item-1").unwrap(),
            RealtimeRequest::retrieve_item("item-1").unwrap(),
            RealtimeRequest::create_response(None).unwrap(),
            RealtimeRequest::cancel_response().unwrap(),
        ];
        let kinds = requests
            .into_iter()
            .map(|request| serde_json::to_value(request).unwrap()["type"].clone())
            .collect::<Vec<_>>();

        assert_eq!(kinds[0], "transcription_session.update");
        assert_eq!(kinds[1], "input_audio_buffer.append");
        assert_eq!(kinds[2], "input_audio_buffer.append_video_frame");
        assert_eq!(kinds[8], "response.cancel");
        assert!(RealtimeRequest::append_audio(&[]).is_err());
        assert!(RealtimeRequest::append_video_frame(&[]).is_err());
        assert!(RealtimeRequest::delete_item(" ").is_err());
        assert!(RealtimeRequest::retrieve_item("").is_err());
    }

    #[test]
    fn forward_compatible_string_enums_and_tool_choice_round_trip() {
        let model: RealtimeModel = serde_json::from_value(json!("glm-realtime-future")).unwrap();
        assert_eq!(model.as_str(), "glm-realtime-future");
        assert_eq!(serde_json::to_value(model).unwrap(), "glm-realtime-future");

        let choice = RealtimeToolChoice::function("weather");
        let value = serde_json::to_value(choice).unwrap();
        assert_eq!(value["type"], "function");
        assert_eq!(value["function"], "weather");
    }
}
