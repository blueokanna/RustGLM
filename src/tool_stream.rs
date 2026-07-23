use std::collections::BTreeMap;
use std::pin::Pin;
use std::task::{Context, Poll};

use async_stream::try_stream;
use futures_util::{Stream, StreamExt};
use serde::{Deserialize, Serialize};

use crate::{
    ChatStream, FunctionCallDelta, ResponseContent, Result, SdkError, ToolCallDelta, Usage,
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FinishReason {
    Stop,
    Length,
    ToolCalls,
    Sensitive,
    NetworkError,
    ModelContextWindowExceeded,
    #[serde(untagged)]
    Other(String),
}

impl From<String> for FinishReason {
    fn from(value: String) -> Self {
        match value.as_str() {
            "stop" => Self::Stop,
            "length" => Self::Length,
            "tool_calls" => Self::ToolCalls,
            "sensitive" => Self::Sensitive,
            "network_error" => Self::NetworkError,
            "model_context_window_exceeded" => Self::ModelContextWindowExceeded,
            _ => Self::Other(value),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolCallStreamDelta {
    pub choice_index: u32,
    pub tool_index: u32,
    pub id: Option<String>,
    pub name: Option<String>,
    pub arguments_delta: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CompletedToolCall {
    pub choice_index: u32,
    pub tool_index: u32,
    pub id: String,
    pub name: String,
    pub arguments: String,
}

impl CompletedToolCall {
    pub fn arguments<T: serde::de::DeserializeOwned>(&self) -> serde_json::Result<T> {
        serde_json::from_str(&self.arguments)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ToolStreamEvent {
    ContentDelta {
        choice_index: u32,
        content: ResponseContent,
    },
    ReasoningDelta {
        choice_index: u32,
        delta: String,
    },
    ToolCallDelta(ToolCallStreamDelta),
    ToolCallCompleted(CompletedToolCall),
    ChoiceCompleted {
        choice_index: u32,
        reason: FinishReason,
    },
    Usage(Usage),
}

/// A zero-disk-I/O stream that assembles Function Call fragments while preserving text and
/// reasoning deltas.
pub struct ToolStream {
    inner: Pin<Box<dyn Stream<Item = Result<ToolStreamEvent>> + Send>>,
}

impl Stream for ToolStream {
    type Item = Result<ToolStreamEvent>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.get_mut().inner.as_mut().poll_next(cx)
    }
}

#[derive(Debug, Default)]
struct ToolCallAssembly {
    id: Option<String>,
    name: Option<String>,
    arguments: String,
}

impl ToolCallAssembly {
    fn apply(&mut self, delta: &ToolCallDelta) -> Result<()> {
        if let Some(kind) = delta.kind.as_deref().filter(|kind| *kind != "function") {
            return Err(SdkError::Stream(
                format!("unsupported streamed tool call type: {kind}").into(),
            ));
        }
        if let Some(id) = delta.id.as_ref().filter(|value| !value.is_empty()) {
            if self.id.as_ref().is_some_and(|current| current != id) {
                return Err(SdkError::Stream(
                    "stream changed a tool call id after it was assigned".into(),
                ));
            }
            self.id = Some(id.clone());
        }
        if let Some(FunctionCallDelta { name, arguments }) = &delta.function {
            if let Some(name) = name.as_ref().filter(|value| !value.is_empty()) {
                if self.name.as_ref().is_some_and(|current| current != name) {
                    return Err(SdkError::Stream(
                        "stream changed a function name after it was assigned".into(),
                    ));
                }
                self.name = Some(name.clone());
            }
            if let Some(arguments) = arguments {
                self.arguments.push_str(arguments);
            }
        }
        Ok(())
    }

    fn complete(self, choice_index: u32, tool_index: u32) -> Result<CompletedToolCall> {
        Ok(CompletedToolCall {
            choice_index,
            tool_index,
            id: self.id.ok_or_else(|| {
                SdkError::Stream(
                    format!(
                        "stream ended before tool call {choice_index}:{tool_index} received an id"
                    )
                    .into(),
                )
            })?,
            name: self.name.ok_or_else(|| {
                SdkError::Stream(
                    format!(
                        "stream ended before tool call {choice_index}:{tool_index} received a name"
                    )
                    .into(),
                )
            })?,
            arguments: self.arguments,
        })
    }
}

pub(crate) fn assemble_tool_stream(mut source: ChatStream) -> ToolStream {
    let stream = try_stream! {
        let mut calls = BTreeMap::<(u32, u32), ToolCallAssembly>::new();

        while let Some(chunk) = source.next().await {
            let chunk = chunk?;
            let mut events = Vec::new();

            for choice in chunk.choices {
                let choice_index = choice.index;
                if let Some(content) = choice.delta.content {
                    events.push(ToolStreamEvent::ContentDelta {
                        choice_index,
                        content,
                    });
                }
                if let Some(delta) = choice.delta.reasoning_content {
                    events.push(ToolStreamEvent::ReasoningDelta {
                        choice_index,
                        delta,
                    });
                }

                for delta in choice.delta.tool_calls {
                    let tool_index = delta.index.unwrap_or(0);
                    calls
                        .entry((choice_index, tool_index))
                        .or_default()
                        .apply(&delta)?;
                    events.push(ToolStreamEvent::ToolCallDelta(ToolCallStreamDelta {
                        choice_index,
                        tool_index,
                        id: delta.id,
                        name: delta.function.as_ref().and_then(|value| value.name.clone()),
                        arguments_delta: delta
                            .function
                            .and_then(|value| value.arguments),
                    }));
                }

                if let Some(reason) = choice.finish_reason {
                    let keys = calls
                        .keys()
                        .filter(|(index, _)| *index == choice_index)
                        .copied()
                        .collect::<Vec<_>>();
                    for key @ (choice_index, tool_index) in keys {
                        let call = calls
                            .remove(&key)
                            .expect("tool call key was collected from the same map")
                            .complete(choice_index, tool_index)?;
                        events.push(ToolStreamEvent::ToolCallCompleted(call));
                    }
                    events.push(ToolStreamEvent::ChoiceCompleted {
                        choice_index,
                        reason: reason.into(),
                    });
                }
            }

            if let Some(usage) = chunk.usage {
                events.push(ToolStreamEvent::Usage(usage));
            }

            for event in events {
                yield event;
            }
        }

        for ((choice_index, tool_index), call) in calls {
            yield ToolStreamEvent::ToolCallCompleted(
                call.complete(choice_index, tool_index)?,
            );
        }
    };
    ToolStream {
        inner: Box::pin(stream),
    }
}

#[cfg(test)]
mod tests {
    use futures_util::{StreamExt, stream};
    use serde::Deserialize;

    use super::*;
    use crate::{ChatChunkChoice, ChatCompletionChunk, ChatDelta};

    fn chunk(choice: ChatChunkChoice) -> ChatCompletionChunk {
        ChatCompletionChunk {
            choices: vec![choice],
            ..ChatCompletionChunk::default()
        }
    }

    #[tokio::test]
    async fn assembles_split_tool_calls_and_preserves_reasoning() {
        let first = chunk(ChatChunkChoice {
            index: 0,
            delta: ChatDelta {
                reasoning_content: Some("checking".into()),
                tool_calls: vec![ToolCallDelta {
                    index: Some(0),
                    id: Some("call-1".into()),
                    kind: Some("function".into()),
                    function: Some(FunctionCallDelta {
                        name: Some("weather".into()),
                        arguments: Some("{\"city\":".into()),
                    }),
                    ..ToolCallDelta::default()
                }],
                ..ChatDelta::default()
            },
            finish_reason: None,
        });
        let second = chunk(ChatChunkChoice {
            index: 0,
            delta: ChatDelta {
                tool_calls: vec![ToolCallDelta {
                    index: Some(0),
                    function: Some(FunctionCallDelta {
                        name: None,
                        arguments: Some("\"Beijing\"}".into()),
                    }),
                    ..ToolCallDelta::default()
                }],
                ..ChatDelta::default()
            },
            finish_reason: Some("tool_calls".into()),
        });
        let source: ChatStream = Box::pin(stream::iter(vec![Ok(first), Ok(second)]));
        let events = assemble_tool_stream(source)
            .collect::<Vec<_>>()
            .await
            .into_iter()
            .collect::<Result<Vec<_>>>()
            .unwrap();

        let completed = events
            .iter()
            .find_map(|event| match event {
                ToolStreamEvent::ToolCallCompleted(call) => Some(call),
                _ => None,
            })
            .unwrap();
        #[derive(Deserialize)]
        struct Arguments {
            city: String,
        }
        assert_eq!(completed.arguments::<Arguments>().unwrap().city, "Beijing");
        assert!(events.iter().any(|event| matches!(
            event,
            ToolStreamEvent::ReasoningDelta { delta, .. } if delta == "checking"
        )));
        assert!(events.iter().any(|event| matches!(
            event,
            ToolStreamEvent::ChoiceCompleted {
                reason: FinishReason::ToolCalls,
                ..
            }
        )));
    }

    #[tokio::test]
    async fn rejects_inconsistent_streamed_identity() {
        let choices = ["first", "second"].into_iter().map(|name| {
            Ok(chunk(ChatChunkChoice {
                index: 0,
                delta: ChatDelta {
                    tool_calls: vec![ToolCallDelta {
                        index: Some(0),
                        id: Some("call-1".into()),
                        function: Some(FunctionCallDelta {
                            name: Some(name.into()),
                            arguments: None,
                        }),
                        ..ToolCallDelta::default()
                    }],
                    ..ChatDelta::default()
                },
                finish_reason: None,
            }))
        });
        let source: ChatStream = Box::pin(stream::iter(choices));
        let errors = assemble_tool_stream(source).collect::<Vec<_>>().await;
        assert!(errors.last().is_some_and(Result::is_err));
    }

    #[tokio::test]
    async fn emits_content_usage_and_completes_calls_at_stream_end() {
        let chunk = ChatCompletionChunk {
            choices: vec![ChatChunkChoice {
                index: 2,
                delta: ChatDelta {
                    content: Some(ResponseContent::Text("answer".into())),
                    tool_calls: vec![ToolCallDelta {
                        index: Some(3),
                        id: Some("call-3".into()),
                        function: Some(FunctionCallDelta {
                            name: Some("lookup".into()),
                            arguments: Some("{}".into()),
                        }),
                        ..ToolCallDelta::default()
                    }],
                    ..ChatDelta::default()
                },
                finish_reason: None,
            }],
            usage: Some(Usage {
                total_tokens: 9,
                ..Usage::default()
            }),
            ..ChatCompletionChunk::default()
        };
        let source: ChatStream = Box::pin(stream::iter([Ok(chunk)]));
        let events = assemble_tool_stream(source)
            .collect::<Vec<_>>()
            .await
            .into_iter()
            .collect::<Result<Vec<_>>>()
            .unwrap();

        assert!(events.iter().any(|event| matches!(
            event,
            ToolStreamEvent::ContentDelta {
                choice_index: 2,
                content: ResponseContent::Text(value)
            } if value == "answer"
        )));
        assert!(events.iter().any(|event| matches!(
            event,
            ToolStreamEvent::Usage(usage) if usage.total_tokens == 9
        )));
        assert!(events.iter().any(|event| matches!(
            event,
            ToolStreamEvent::ToolCallCompleted(call)
                if call.tool_index == 3 && call.name == "lookup"
        )));
    }

    #[tokio::test]
    async fn rejects_non_function_and_incomplete_tool_calls() {
        let invalid_kind = chunk(ChatChunkChoice {
            index: 0,
            delta: ChatDelta {
                tool_calls: vec![ToolCallDelta {
                    index: Some(0),
                    kind: Some("retrieval".into()),
                    ..ToolCallDelta::default()
                }],
                ..ChatDelta::default()
            },
            finish_reason: None,
        });
        let source: ChatStream = Box::pin(stream::iter([Ok(invalid_kind)]));
        assert!(
            assemble_tool_stream(source)
                .collect::<Vec<_>>()
                .await
                .last()
                .is_some_and(Result::is_err)
        );

        let incomplete = chunk(ChatChunkChoice {
            index: 0,
            delta: ChatDelta {
                tool_calls: vec![ToolCallDelta {
                    index: Some(0),
                    function: Some(FunctionCallDelta {
                        name: Some("lookup".into()),
                        arguments: Some("{}".into()),
                    }),
                    ..ToolCallDelta::default()
                }],
                ..ChatDelta::default()
            },
            finish_reason: Some("tool_calls".into()),
        });
        let source: ChatStream = Box::pin(stream::iter([Ok(incomplete)]));
        assert!(
            assemble_tool_stream(source)
                .collect::<Vec<_>>()
                .await
                .last()
                .is_some_and(Result::is_err)
        );
    }
}
