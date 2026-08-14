use std::marker::PhantomData;

use crate::{
    ChatCompletionRequest, ChatMessage, ContentPart, MessageRole, ReasoningEffort, Thinking, Tool,
};

mod sealed {
    pub trait Sealed {}
}

/// Capabilities fixed by a built-in model marker.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ModelCapabilities {
    pub vision: bool,
    pub thinking: bool,
    pub reasoning_effort: bool,
    pub tools: bool,
    pub tool_stream: bool,
}

/// A built-in Zhipu chat model with compile-time capability metadata.
///
/// This trait is sealed so downstream crates cannot accidentally claim a capability for an
/// arbitrary model identifier. Use [`ChatCompletionRequest`] when a newly released or private
/// model is not represented here yet.
pub trait ChatModel: sealed::Sealed + Send + Sync + 'static {
    const ID: &'static str;
    const CAPABILITIES: ModelCapabilities;
}

pub trait TextChatModel: ChatModel {}
pub trait VisionChatModel: ChatModel {}
pub trait SupportsThinking: ChatModel {}
pub trait SupportsReasoningEffort: SupportsThinking {}
pub trait SupportsTools: ChatModel {}
pub trait SupportsToolStream: SupportsTools {}

macro_rules! model_marker {
    ($name:ident, $id:literal, $capabilities:expr) => {
        #[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
        pub struct $name;

        impl sealed::Sealed for $name {}

        impl ChatModel for $name {
            const ID: &'static str = $id;
            const CAPABILITIES: ModelCapabilities = $capabilities;
        }
    };
}

const fn text_capabilities(
    thinking: bool,
    reasoning_effort: bool,
    tool_stream: bool,
) -> ModelCapabilities {
    ModelCapabilities {
        vision: false,
        thinking,
        reasoning_effort,
        tools: true,
        tool_stream,
    }
}

const fn vision_capabilities(thinking: bool) -> ModelCapabilities {
    ModelCapabilities {
        vision: true,
        thinking,
        reasoning_effort: false,
        tools: true,
        tool_stream: false,
    }
}

model_marker!(Glm53, "glm-5.3", text_capabilities(true, true, true));
model_marker!(Glm52, "glm-5.2", text_capabilities(true, true, true));
model_marker!(Glm51, "glm-5.1", text_capabilities(true, false, true));
model_marker!(
    Glm51Highspeed,
    "glm-5.1-highspeed",
    text_capabilities(true, false, true)
);
model_marker!(
    Glm5Turbo,
    "glm-5-turbo",
    text_capabilities(true, false, true)
);
model_marker!(Glm5, "glm-5", text_capabilities(true, false, true));
model_marker!(Glm47, "glm-4.7", text_capabilities(true, false, true));
model_marker!(
    Glm47Flash,
    "glm-4.7-flash",
    text_capabilities(true, false, false)
);
model_marker!(
    Glm47FlashX,
    "glm-4.7-flashx",
    text_capabilities(true, false, false)
);
model_marker!(Glm46, "glm-4.6", text_capabilities(true, false, true));
model_marker!(
    Glm45Air,
    "glm-4.5-air",
    text_capabilities(true, false, false)
);
model_marker!(
    Glm45AirX,
    "glm-4.5-airx",
    text_capabilities(true, false, false)
);
model_marker!(
    Glm45Flash,
    "glm-4.5-flash",
    text_capabilities(true, false, false)
);
model_marker!(
    Glm4Flash250414,
    "glm-4-flash-250414",
    text_capabilities(false, false, false)
);
model_marker!(
    Glm4FlashX250414,
    "glm-4-flashx-250414",
    text_capabilities(false, false, false)
);

model_marker!(Glm5vTurbo, "glm-5v-turbo", vision_capabilities(true));
model_marker!(AutoGlmPhone, "autoglm-phone", vision_capabilities(false));
model_marker!(Glm46v, "glm-4.6v", vision_capabilities(true));
model_marker!(Glm46vFlash, "glm-4.6v-flash", vision_capabilities(true));
model_marker!(Glm46vFlashX, "glm-4.6v-flashx", vision_capabilities(true));
model_marker!(Glm4vFlash, "glm-4v-flash", vision_capabilities(false));
model_marker!(
    Glm41vThinkingFlash,
    "glm-4.1v-thinking-flash",
    vision_capabilities(true)
);
model_marker!(
    Glm41vThinkingFlashX,
    "glm-4.1v-thinking-flashx",
    vision_capabilities(true)
);

macro_rules! impl_text {
    ($($model:ty),+ $(,)?) => { $(impl TextChatModel for $model {})+ };
}

macro_rules! impl_vision {
    ($($model:ty),+ $(,)?) => { $(impl VisionChatModel for $model {})+ };
}

macro_rules! impl_thinking {
    ($($model:ty),+ $(,)?) => { $(impl SupportsThinking for $model {})+ };
}

macro_rules! impl_tools {
    ($($model:ty),+ $(,)?) => { $(impl SupportsTools for $model {})+ };
}

macro_rules! impl_tool_stream {
    ($($model:ty),+ $(,)?) => { $(impl SupportsToolStream for $model {})+ };
}

impl_text!(
    Glm53,
    Glm52,
    Glm51,
    Glm51Highspeed,
    Glm5Turbo,
    Glm5,
    Glm47,
    Glm47Flash,
    Glm47FlashX,
    Glm46,
    Glm45Air,
    Glm45AirX,
    Glm45Flash,
    Glm4Flash250414,
    Glm4FlashX250414,
);
impl_vision!(
    Glm5vTurbo,
    AutoGlmPhone,
    Glm46v,
    Glm46vFlash,
    Glm46vFlashX,
    Glm4vFlash,
    Glm41vThinkingFlash,
    Glm41vThinkingFlashX,
);
impl_thinking!(
    Glm53,
    Glm52,
    Glm51,
    Glm51Highspeed,
    Glm5Turbo,
    Glm5,
    Glm47,
    Glm47Flash,
    Glm47FlashX,
    Glm46,
    Glm45Air,
    Glm45AirX,
    Glm45Flash,
    Glm5vTurbo,
    Glm46v,
    Glm46vFlash,
    Glm46vFlashX,
    Glm41vThinkingFlash,
    Glm41vThinkingFlashX,
);
impl SupportsReasoningEffort for Glm53 {}
impl SupportsReasoningEffort for Glm52 {}
impl_tools!(
    Glm53,
    Glm52,
    Glm51,
    Glm51Highspeed,
    Glm5Turbo,
    Glm5,
    Glm47,
    Glm47Flash,
    Glm47FlashX,
    Glm46,
    Glm45Air,
    Glm45AirX,
    Glm45Flash,
    Glm4Flash250414,
    Glm4FlashX250414,
    Glm5vTurbo,
    AutoGlmPhone,
    Glm46v,
    Glm46vFlash,
    Glm46vFlashX,
    Glm4vFlash,
    Glm41vThinkingFlash,
    Glm41vThinkingFlashX,
);
impl_tool_stream!(
    Glm53,
    Glm52,
    Glm51,
    Glm51Highspeed,
    Glm5Turbo,
    Glm5,
    Glm47,
    Glm46
);

mod request_state {
    pub trait Sealed {}

    #[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
    pub struct MissingInput;

    #[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
    pub struct Ready;

    impl Sealed for MissingInput {}
    impl Sealed for Ready {}
}

pub use request_state::{MissingInput, Ready};

/// A capability-checked chat request.
///
/// `M` determines which model-specific methods exist, while `S` prevents sending a request before
/// it contains user or tool input. Convert with [`TypedChatRequest::into_raw`] when a raw extension
/// field is required.
#[derive(Debug, Clone, PartialEq)]
pub struct TypedChatRequest<M: ChatModel, S: request_state::Sealed = MissingInput> {
    inner: ChatCompletionRequest,
    marker: PhantomData<(M, S)>,
}

impl<M: ChatModel> Default for TypedChatRequest<M, MissingInput> {
    fn default() -> Self {
        Self::new()
    }
}

impl<M: ChatModel> TypedChatRequest<M, MissingInput> {
    pub fn new() -> Self {
        Self {
            inner: ChatCompletionRequest::new(M::ID),
            marker: PhantomData,
        }
    }
}

impl<M: ChatModel, S: request_state::Sealed> TypedChatRequest<M, S> {
    pub const fn model_id(&self) -> &'static str {
        M::ID
    }

    pub const fn capabilities(&self) -> ModelCapabilities {
        M::CAPABILITIES
    }

    pub fn system(mut self, value: impl Into<String>) -> Self {
        self.inner.messages.push(ChatMessage::system(value));
        self
    }

    pub fn assistant(mut self, value: impl Into<String>) -> Self {
        self.inner.messages.push(ChatMessage::assistant(value));
        self
    }

    pub fn temperature(mut self, value: f32) -> Self {
        self.inner.temperature = Some(value);
        self
    }

    pub fn top_p(mut self, value: f32) -> Self {
        self.inner.top_p = Some(value);
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

    pub fn into_raw(self) -> ChatCompletionRequest {
        self.inner
    }

    fn transition<T: request_state::Sealed>(self) -> TypedChatRequest<M, T> {
        TypedChatRequest {
            inner: self.inner,
            marker: PhantomData,
        }
    }
}

impl<M: TextChatModel, S: request_state::Sealed> TypedChatRequest<M, S> {
    pub fn user(mut self, value: impl Into<String>) -> TypedChatRequest<M, Ready> {
        self.inner.messages.push(ChatMessage::user(value));
        self.transition()
    }
}

impl<M: VisionChatModel, S: request_state::Sealed> TypedChatRequest<M, S> {
    pub fn user_parts(
        mut self,
        parts: impl IntoIterator<Item = ContentPart>,
    ) -> TypedChatRequest<M, Ready> {
        self.inner.messages.push(ChatMessage::multimodal(
            MessageRole::User,
            parts.into_iter().collect(),
        ));
        self.transition()
    }
}

impl<M: SupportsThinking, S: request_state::Sealed> TypedChatRequest<M, S> {
    pub fn thinking(mut self, value: Thinking) -> Self {
        self.inner.thinking = Some(value);
        self
    }
}

impl<M: SupportsReasoningEffort, S: request_state::Sealed> TypedChatRequest<M, S> {
    /// Sets GLM-5.3 (or GLM-5.2) reasoning effort.
    ///
    /// This method does not exist for models that do not declare the capability:
    ///
    /// ```compile_fail
    /// use rustglm::{Glm51, ReasoningEffort, TypedChatRequest};
    ///
    /// let _ = TypedChatRequest::<Glm51>::new()
    ///     .reasoning_effort(ReasoningEffort::High);
    /// ```
    pub fn reasoning_effort(mut self, value: ReasoningEffort) -> Self {
        self.inner.reasoning_effort = Some(value);
        self
    }
}

impl<M: SupportsTools, S: request_state::Sealed> TypedChatRequest<M, S> {
    pub fn tool(mut self, value: Tool) -> Self {
        self.inner.tools.get_or_insert_with(Vec::new).push(value);
        self
    }

    pub fn tools(mut self, values: impl IntoIterator<Item = Tool>) -> Self {
        self.inner.tools.get_or_insert_with(Vec::new).extend(values);
        self
    }

    pub fn tool_result(
        mut self,
        call_id: impl Into<String>,
        output: impl Into<String>,
    ) -> TypedChatRequest<M, Ready> {
        self.inner
            .messages
            .push(ChatMessage::tool_result(call_id, output));
        self.transition()
    }
}

impl<M: SupportsToolStream, S: request_state::Sealed> TypedChatRequest<M, S> {
    /// Enables incremental Function Call output for models that support `tool_stream`.
    ///
    /// ```compile_fail
    /// use rustglm::{Glm45Air, TypedChatRequest};
    ///
    /// let _ = TypedChatRequest::<Glm45Air>::new().tool_stream();
    /// ```
    pub fn tool_stream(mut self) -> Self {
        self.inner.tool_stream = Some(true);
        self
    }
}

impl<M: ChatModel> TypedChatRequest<M, Ready> {
    pub fn as_raw(&self) -> &ChatCompletionRequest {
        &self.inner
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn typed_glm53_request_sets_capability_checked_fields() {
        let request = TypedChatRequest::<Glm53>::new()
            .system("be precise")
            .thinking(Thinking::enabled())
            .reasoning_effort(ReasoningEffort::Max)
            .tool_stream()
            .user("hello");

        assert_eq!(request.model_id(), "glm-5.3");
        assert_eq!(request.as_raw().model, "glm-5.3");
        assert_eq!(request.as_raw().messages.len(), 2);
        assert_eq!(request.as_raw().tool_stream, Some(true));
        assert!(request.capabilities().reasoning_effort);
        assert!(request.capabilities().tools);
        assert!(!request.capabilities().vision);
    }

    #[test]
    fn typed_glm52_request_sets_capability_checked_fields() {
        let request = TypedChatRequest::<Glm52>::new()
            .system("be precise")
            .thinking(Thinking::enabled())
            .reasoning_effort(ReasoningEffort::High)
            .tool_stream()
            .user("hello");

        assert_eq!(request.model_id(), "glm-5.2");
        assert_eq!(request.as_raw().model, "glm-5.2");
        assert_eq!(request.as_raw().messages.len(), 2);
        assert_eq!(request.as_raw().tool_stream, Some(true));
        assert!(request.capabilities().reasoning_effort);
    }

    #[test]
    fn typed_vision_request_accepts_multimodal_input() {
        let request = TypedChatRequest::<Glm5vTurbo>::new().user_parts([
            ContentPart::image_url("https://example.com/image.png"),
            ContentPart::text("describe it"),
        ]);

        assert!(request.capabilities().vision);
        assert!(matches!(
            &request.as_raw().messages[0].content,
            Some(crate::MessageContent::Parts(parts)) if parts.len() == 2
        ));
    }

    fn model_descriptor<M: ChatModel>() -> (&'static str, ModelCapabilities) {
        (M::ID, M::CAPABILITIES)
    }

    #[test]
    fn every_official_marker_has_the_expected_wire_id() {
        let models = [
            model_descriptor::<Glm53>(),
            model_descriptor::<Glm52>(),
            model_descriptor::<Glm51>(),
            model_descriptor::<Glm51Highspeed>(),
            model_descriptor::<Glm5Turbo>(),
            model_descriptor::<Glm5>(),
            model_descriptor::<Glm47>(),
            model_descriptor::<Glm47Flash>(),
            model_descriptor::<Glm47FlashX>(),
            model_descriptor::<Glm46>(),
            model_descriptor::<Glm45Air>(),
            model_descriptor::<Glm45AirX>(),
            model_descriptor::<Glm45Flash>(),
            model_descriptor::<Glm4Flash250414>(),
            model_descriptor::<Glm4FlashX250414>(),
            model_descriptor::<Glm5vTurbo>(),
            model_descriptor::<AutoGlmPhone>(),
            model_descriptor::<Glm46v>(),
            model_descriptor::<Glm46vFlash>(),
            model_descriptor::<Glm46vFlashX>(),
            model_descriptor::<Glm4vFlash>(),
            model_descriptor::<Glm41vThinkingFlash>(),
            model_descriptor::<Glm41vThinkingFlashX>(),
        ];
        let ids = models.iter().map(|(id, _)| *id).collect::<Vec<_>>();
        assert_eq!(
            ids,
            [
                "glm-5.3",
                "glm-5.2",
                "glm-5.1",
                "glm-5.1-highspeed",
                "glm-5-turbo",
                "glm-5",
                "glm-4.7",
                "glm-4.7-flash",
                "glm-4.7-flashx",
                "glm-4.6",
                "glm-4.5-air",
                "glm-4.5-airx",
                "glm-4.5-flash",
                "glm-4-flash-250414",
                "glm-4-flashx-250414",
                "glm-5v-turbo",
                "autoglm-phone",
                "glm-4.6v",
                "glm-4.6v-flash",
                "glm-4.6v-flashx",
                "glm-4v-flash",
                "glm-4.1v-thinking-flash",
                "glm-4.1v-thinking-flashx",
            ]
        );
        assert!(models[0].1.reasoning_effort);
        assert!(models[0].1.tool_stream);
        assert!(models[1].1.reasoning_effort);
        assert!(models[1].1.tool_stream);
        assert!(models[15].1.vision);
        assert!(!models[15].1.tool_stream);
        assert!(!model_descriptor::<Glm47Flash>().1.tool_stream);
    }

    #[test]
    fn typed_builder_covers_common_tool_and_transition_paths() {
        let tool = Tool::configured("web_search", "web_search", nextjson::json!({}));
        let request = TypedChatRequest::<Glm52>::new()
            .system("system")
            .assistant("assistant")
            .temperature(0.3)
            .top_p(0.8)
            .max_tokens(100)
            .request_id("request-id")
            .user_id("user-id")
            .tools([tool.clone()])
            .tool(tool)
            .user("question");
        assert_eq!(request.as_raw().tools.as_ref().unwrap().len(), 2);
        assert_eq!(request.as_raw().temperature, Some(0.3));
        assert_eq!(request.into_raw().model, "glm-5.2");

        let request = TypedChatRequest::<Glm52>::new().tool_result("call-1", "output");
        assert_eq!(
            request.as_raw().messages[0].tool_call_id.as_deref(),
            Some("call-1")
        );
    }
}
