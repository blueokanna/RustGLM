use std::pin::Pin;

use async_trait::async_trait;
use futures_util::Stream;

use crate::{ChatCompletionChunk, ChatCompletionRequest, ChatCompletionResponse, Result};

pub type ChatStream = Pin<Box<dyn Stream<Item = Result<ChatCompletionChunk>> + Send>>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProviderCapabilities {
    pub streaming: bool,
    pub multimodal: bool,
    pub tools: bool,
    pub embeddings: bool,
    pub images: bool,
    pub audio: bool,
    pub video: bool,
    pub files: bool,
    pub batches: bool,
    pub realtime: bool,
}

impl ProviderCapabilities {
    pub const fn zhipu() -> Self {
        Self {
            streaming: true,
            multimodal: true,
            tools: true,
            embeddings: true,
            images: true,
            audio: true,
            video: true,
            files: true,
            batches: true,
            realtime: cfg!(feature = "realtime"),
        }
    }

    pub const fn openai_compatible() -> Self {
        Self {
            streaming: true,
            multimodal: true,
            tools: true,
            embeddings: false,
            images: false,
            audio: false,
            video: false,
            files: false,
            batches: false,
            realtime: false,
        }
    }
}

#[async_trait]
pub trait ChatProvider: Send + Sync {
    fn name(&self) -> &str;
    fn capabilities(&self) -> ProviderCapabilities;
    async fn complete(&self, request: ChatCompletionRequest) -> Result<ChatCompletionResponse>;
    async fn stream(&self, request: ChatCompletionRequest) -> Result<ChatStream>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zhipu_capabilities_are_complete() {
        let value = ProviderCapabilities::zhipu();
        assert!(value.streaming);
        assert!(value.multimodal);
        assert!(value.tools);
        assert!(value.embeddings);
        assert!(value.images);
        assert!(value.audio);
        assert!(value.video);
        assert!(value.files);
        assert!(value.batches);
        assert_eq!(value.realtime, cfg!(feature = "realtime"));
    }

    #[test]
    fn compatible_capabilities_do_not_overclaim() {
        let value = ProviderCapabilities::openai_compatible();
        assert!(value.streaming);
        assert!(value.multimodal);
        assert!(value.tools);
        assert!(!value.embeddings);
        assert!(!value.images);
        assert!(!value.audio);
        assert!(!value.video);
        assert!(!value.files);
        assert!(!value.batches);
        assert!(!value.realtime);
    }
}
