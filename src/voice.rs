use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use nextjson::{NsonDeserialize as Deserialize, NsonSerialize as Serialize};

use crate::{
    ChatCompletionRequest, ChatCompletionResponse, ChatMessage, ContentPart, MessageRole, Result,
    SdkError,
};

pub const GLM_4_VOICE_MODEL: &str = "glm-4-voice";
pub const GLM_4_VOICE_OUTPUT_SAMPLE_RATE: u32 = 44_100;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(transparent)]
pub struct Glm4VoiceRequest {
    inner: ChatCompletionRequest,
}

impl Default for Glm4VoiceRequest {
    fn default() -> Self {
        Self::new()
    }
}

impl Glm4VoiceRequest {
    pub fn new() -> Self {
        Self {
            inner: ChatCompletionRequest::new(GLM_4_VOICE_MODEL),
        }
    }

    pub fn from_wav(prompt: impl Into<String>, wav: &[u8]) -> Result<Self> {
        if wav.is_empty() {
            return Err(SdkError::Validation(
                "GLM-4-Voice input WAV cannot be empty".into(),
            ));
        }
        let prompt = prompt.into();
        if prompt.trim().is_empty() {
            return Err(SdkError::Validation(
                "GLM-4-Voice prompt cannot be empty".into(),
            ));
        }
        Ok(Self::new().message(ChatMessage::multimodal(
            MessageRole::User,
            vec![
                ContentPart::text(prompt),
                ContentPart::input_audio(STANDARD.encode(wav), "wav"),
            ],
        )))
    }

    pub fn message(mut self, value: ChatMessage) -> Self {
        self.inner.messages.push(value);
        self
    }

    pub fn temperature(mut self, value: f32) -> Self {
        self.inner.temperature = Some(value);
        self
    }

    pub fn max_tokens(mut self, value: u32) -> Self {
        self.inner.max_tokens = Some(value);
        self
    }

    pub fn request_id(mut self, value: impl Into<String>) -> Self {
        self.inner.request_id = Some(value.into());
        self
    }

    pub fn user_id(mut self, value: impl Into<String>) -> Self {
        self.inner.user_id = Some(value.into());
        self
    }

    pub fn as_chat_request(&self) -> &ChatCompletionRequest {
        &self.inner
    }

    pub fn into_chat_request(self) -> ChatCompletionRequest {
        self.inner
    }
}

impl ChatCompletionResponse {
    pub fn audio_bytes(&self) -> Result<Option<Vec<u8>>> {
        self.choices
            .first()
            .and_then(|choice| choice.message.audio.as_ref())
            .and_then(|audio| audio.data.as_deref())
            .map(|data| {
                STANDARD.decode(data).map_err(|error| SdkError::Decode {
                    message: format!("GLM-4-Voice audio is not valid base64: {error}"),
                    body: data.to_owned(),
                })
            })
            .transpose()
    }

    pub fn audio_wav(&self) -> Result<Option<Vec<u8>>> {
        self.audio_bytes()?
            .map(|pcm| pcm16_mono_wav(&pcm, GLM_4_VOICE_OUTPUT_SAMPLE_RATE))
            .transpose()
    }
}

pub fn pcm16_mono_wav(pcm: &[u8], sample_rate: u32) -> Result<Vec<u8>> {
    #[allow(clippy::manual_is_multiple_of)]
    if pcm.len() % 2 != 0 {
        return Err(SdkError::Validation(
            "PCM16 audio must contain an even number of bytes".into(),
        ));
    }
    if sample_rate == 0 {
        return Err(SdkError::Validation(
            "WAV sample rate must be greater than zero".into(),
        ));
    }
    let data_size = u32::try_from(pcm.len())
        .map_err(|_| SdkError::Validation("PCM audio is too large for a WAV file".into()))?;
    let riff_size = data_size
        .checked_add(36)
        .ok_or_else(|| SdkError::Validation("WAV size overflow".into()))?;
    let byte_rate = sample_rate
        .checked_mul(2)
        .ok_or_else(|| SdkError::Validation("WAV byte rate overflow".into()))?;
    let mut wav = Vec::with_capacity(pcm.len() + 44);
    wav.extend_from_slice(b"RIFF");
    wav.extend_from_slice(&riff_size.to_le_bytes());
    wav.extend_from_slice(b"WAVEfmt ");
    wav.extend_from_slice(&16u32.to_le_bytes());
    wav.extend_from_slice(&1u16.to_le_bytes());
    wav.extend_from_slice(&1u16.to_le_bytes());
    wav.extend_from_slice(&sample_rate.to_le_bytes());
    wav.extend_from_slice(&byte_rate.to_le_bytes());
    wav.extend_from_slice(&2u16.to_le_bytes());
    wav.extend_from_slice(&16u16.to_le_bytes());
    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&data_size.to_le_bytes());
    wav.extend_from_slice(pcm);
    Ok(wav)
}

#[cfg(test)]
mod tests {
    use nextjson::json;

    use super::*;

    #[test]
    fn voice_request_encodes_wav_using_official_wire_format() {
        let request = Glm4VoiceRequest::from_wav("repeat slowly", b"RIFFdata").unwrap();
        let value = nextjson::to_value(&request).unwrap();
        assert_eq!(value["model"].as_str(), Some("glm-4-voice"));
        assert_eq!(
            value["messages"][0]["content"][1]["type"].as_str(),
            Some("input_audio")
        );
        assert_eq!(
            value["messages"][0]["content"][1]["input_audio"]["format"].as_str(),
            Some("wav")
        );
        assert_eq!(
            value["messages"][0]["content"][1]["input_audio"]["data"].as_str(),
            Some("UklGRmRhdGE=")
        );
    }

    #[test]
    fn voice_response_decodes_pcm_and_wraps_wav() {
        let response: ChatCompletionResponse = nextjson::from_value(json!({
            "model":"glm-4-voice",
            "choices":[{"message":{"role":"assistant","content":"ok","audio":{
                "id":"audio-1","expires_at":1749187238,"data":"AQIDBA=="
            }}}]
        }))
        .unwrap();
        assert_eq!(response.audio_bytes().unwrap(), Some(vec![1, 2, 3, 4]));
        let wav = response.audio_wav().unwrap().unwrap();
        assert_eq!(&wav[..4], b"RIFF");
        assert_eq!(&wav[8..12], b"WAVE");
        assert_eq!(&wav[44..], &[1, 2, 3, 4]);
    }

    #[test]
    fn voice_validation_rejects_invalid_input_and_audio() {
        assert!(Glm4VoiceRequest::from_wav("", b"wav").is_err());
        assert!(Glm4VoiceRequest::from_wav("prompt", b"").is_err());
        assert!(pcm16_mono_wav(&[1], 44_100).is_err());
        assert!(pcm16_mono_wav(&[1, 2], 0).is_err());
        let response: ChatCompletionResponse = nextjson::from_value(json!({
            "choices":[{"message":{"audio":{"data":"%%%"}}}]
        }))
        .unwrap();
        assert!(response.audio_bytes().is_err());
    }

    #[test]
    fn voice_builder_and_empty_response_helpers_are_complete() {
        let request = Glm4VoiceRequest::new()
            .message(ChatMessage::user("text"))
            .temperature(0.5)
            .max_tokens(256)
            .request_id("request-id")
            .user_id("user-id");
        assert_eq!(request.as_chat_request().temperature, Some(0.5));
        let request = request.into_chat_request();
        assert_eq!(request.max_tokens, Some(256));
        assert_eq!(request.request_id.as_deref(), Some("request-id"));
        assert_eq!(request.user_id.as_deref(), Some("user-id"));
        let response = ChatCompletionResponse::default();
        assert_eq!(response.audio_bytes().unwrap(), None);
        assert_eq!(response.audio_wav().unwrap(), None);
        let wav = pcm16_mono_wav(&[0, 0, 1, 0], 8_000).unwrap();
        assert_eq!(u32::from_le_bytes(wav[24..28].try_into().unwrap()), 8_000);
        assert_eq!(u32::from_le_bytes(wav[28..32].try_into().unwrap()), 16_000);
    }
}
