//! Zero-copy Server-Sent Events (SSE) decoding shared by the chat, ToolStream, and
//! agent streaming paths.
//!
//! Incoming bytes are appended to an internal buffer exactly once; complete lines are located
//! by index scanning and processed as `&[u8]` slices, so no per-line `Vec` is allocated and the
//! consumed prefix is not memmoved per event. The common single-`data:`-line event is
//! deserialized directly from the buffer slice, avoiding a second copy of the payload. Events
//! spanning multiple `data:` lines are joined with `\n` per the SSE spec, and the buffer is
//! compacted in amortized O(1) fashion so long streams stay memory-bounded.

use std::marker::PhantomData;

use nextjson::NsonDeserialize as Deserialize;

use crate::security::{DEFAULT_MAX_SSE_DATA_LINES, DEFAULT_MAX_SSE_EVENT_BYTES};
use crate::{Result, SdkError};

/// Upper bound for a single in-flight SSE event. Guards against unbounded buffer growth when a
/// peer sends a pathologically long line without a terminator.
const MAX_EVENT_BYTES: usize = DEFAULT_MAX_SSE_EVENT_BYTES;

/// Streaming SSE decoder with zero-copy line parsing and bounded memory.
pub(crate) struct SseDecoder<T> {
    buffer: Vec<u8>,
    consumed: usize,
    /// Joined payload of the in-flight event, used only when it spans multiple `data:` lines.
    event: Vec<u8>,
    /// Buffer range of the current single-line `data:` payload while it is still contiguous.
    single: Option<(usize, usize)>,
    data_lines: u32,
    done: bool,
    marker: PhantomData<T>,
}

impl<T> Default for SseDecoder<T> {
    fn default() -> Self {
        Self {
            buffer: Vec::new(),
            consumed: 0,
            event: Vec::new(),
            single: None,
            data_lines: 0,
            done: false,
            marker: PhantomData,
        }
    }
}

impl<T: for<'de> Deserialize<'de>> SseDecoder<T> {
    /// Compact the buffer once the consumed prefix exceeds this size.
    const COMPACT_THRESHOLD: usize = 8 * 1024;

    pub(crate) fn push(&mut self, bytes: &[u8]) -> Result<Vec<T>> {
        if self.done {
            return Ok(Vec::new());
        }
        let in_flight = self.buffer.len().saturating_add(self.event.len());
        if in_flight.saturating_add(bytes.len()) > MAX_EVENT_BYTES {
            return Err(SdkError::Stream(
                "SSE event exceeds the maximum supported size".into(),
            ));
        }
        self.buffer.extend_from_slice(bytes);
        self.drain(false)
    }

    pub(crate) fn finish(&mut self) -> Result<Vec<T>> {
        let values = self.drain(true)?;
        self.done = true;
        Ok(values)
    }

    fn drain(&mut self, finish: bool) -> Result<Vec<T>> {
        let mut values = Vec::new();
        let mut cursor = self.consumed;
        while let Some(position) = self.buffer[cursor..].iter().position(|byte| *byte == b'\n') {
            let line_start = cursor;
            let line_end = line_start + position;
            let line = trim_cr(&self.buffer[line_start..line_end]);
            cursor = line_end + 1;
            if line.is_empty() {
                self.consume_event(&mut values)?;
                if self.done {
                    break;
                }
            } else if let Some((start, end)) = data_range(line, line_start + line.len()) {
                self.append_data(start, end)?;
            }
        }
        if finish {
            if cursor < self.buffer.len() {
                let line = trim_cr(&self.buffer[cursor..]);
                if let Some((start, end)) = data_range(line, cursor + line.len()) {
                    self.append_data(start, end)?;
                }
                cursor = self.buffer.len();
            }
            self.consume_event(&mut values)?;
        }
        self.consumed = cursor;
        // Never compact while a single-line payload still references the buffer; delayed
        // compaction bounds memory while keeping in-flight slices valid.
        if self.single.is_none() && self.consumed >= Self::COMPACT_THRESHOLD {
            self.buffer.drain(..self.consumed);
            self.consumed = 0;
        }
        Ok(values)
    }

    fn append_data(&mut self, start: usize, end: usize) -> Result<()> {
        if self.data_lines >= DEFAULT_MAX_SSE_DATA_LINES as u32 {
            return Err(SdkError::Stream(
                "SSE event exceeds the maximum supported data line count".into(),
            ));
        }
        self.data_lines += 1;
        if let Some((first_start, first_end)) = self.single.take() {
            // A second `data:` line arrived: materialize the deferred first payload.
            let first = &self.buffer[first_start..first_end];
            self.event.extend_from_slice(first);
        }
        if self.event.is_empty() {
            self.single = Some((start, end));
        } else {
            // SSE joins every `data:` line with a single `\n`.
            self.event.push(b'\n');
            let payload = &self.buffer[start..end];
            self.event.extend_from_slice(payload);
        }
        Ok(())
    }

    fn consume_event(&mut self, values: &mut Vec<T>) -> Result<()> {
        self.data_lines = 0;
        if let Some((start, end)) = self.single.take() {
            if start == end {
                return Ok(());
            }
            // Zero-copy: deserialize straight from the live buffer slice.
            let payload = &self.buffer[start..end];
            if payload == b"[DONE]" {
                self.done = true;
                return Ok(());
            }
            let value = nextjson::from_slice(payload).map_err(|error| {
                SdkError::Stream(format!("{}: {}", error, String::from_utf8_lossy(payload)).into())
            })?;
            values.push(value);
            return Ok(());
        }
        let payload = std::mem::take(&mut self.event);
        if payload.is_empty() {
            return Ok(());
        }
        if payload == b"[DONE]" {
            self.done = true;
            return Ok(());
        }
        let value = nextjson::from_slice(&payload).map_err(|error| {
            SdkError::Stream(format!("{}: {}", error, String::from_utf8_lossy(&payload)).into())
        })?;
        values.push(value);
        Ok(())
    }
}

/// Strips a trailing carriage return from an SSE line.
fn trim_cr(line: &[u8]) -> &[u8] {
    match line.last() {
        Some(b'\r') => &line[..line.len() - 1],
        _ => line,
    }
}

/// Returns the buffer range `[start, end)` of the payload of an SSE `data:` line, or
/// `None` for any other field. Returning indices instead of a slice keeps the caller
/// free to mutate the decoder after the borrow ends.
fn data_range(line: &[u8], end: usize) -> Option<(usize, usize)> {
    let mut payload = line.strip_prefix(b"data:")?;
    if payload.first() == Some(&b' ') {
        payload = &payload[1..];
    }
    Some((end - payload.len(), end))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, nextjson::NsonDeserialize)]
    struct Message {
        id: String,
    }

    fn decoder() -> SseDecoder<Message> {
        SseDecoder::default()
    }

    #[test]
    fn parses_single_and_multi_line_events_zero_copy() {
        let mut decoder = decoder();
        let values = decoder
            .push(b": ping\r\nevent: message\r\ndata: {\"id\":\"one\",\r\ndata: \"ignored\":true}\r\n\r\n")
            .unwrap();
        assert_eq!(values[0].id, "one");
        assert!(decoder.push(b"data: [DONE]\n\n").unwrap().is_empty());
        assert!(decoder.push(b"data: {bad}\n\n").unwrap().is_empty());
        assert!(decoder.finish().unwrap().is_empty());
    }

    #[test]
    fn flushes_trailing_line_without_terminator() {
        let mut decoder = decoder();
        assert!(decoder.push(b"data:{\"id\":\"two\"}").unwrap().is_empty());
        assert_eq!(decoder.finish().unwrap()[0].id, "two");
    }

    #[test]
    fn handles_split_utf8_across_frames() {
        let json = nextjson::to_vec(&nextjson::json!({"id":"你好"})).unwrap();
        let mut frame = b"data: ".to_vec();
        frame.extend(json);
        frame.extend_from_slice(b"\n\ndata: [DONE]\n\n");
        let split = frame.iter().position(|value| *value >= 0x80).unwrap() + 1;
        let mut decoder = decoder();
        assert!(decoder.push(&frame[..split]).unwrap().is_empty());
        assert_eq!(decoder.push(&frame[split..]).unwrap()[0].id, "你好");
    }

    #[test]
    fn rejects_oversized_events_without_unbounded_growth() {
        let mut decoder = decoder();
        let oversized = vec![b'a'; MAX_EVENT_BYTES + 1];
        assert!(matches!(decoder.push(&oversized), Err(SdkError::Stream(_))));
    }

    #[test]
    fn rejects_oversized_multi_line_events() {
        let mut decoder = decoder();
        let first = vec![b'b'; MAX_EVENT_BYTES / 2];
        let mut line = b"data: ".to_vec();
        line.extend_from_slice(&first);
        line.extend_from_slice(b"\n");
        assert!(decoder.push(&line).unwrap().is_empty());
        let second = vec![b'c'; MAX_EVENT_BYTES / 2];
        let mut line = b"data: ".to_vec();
        line.extend_from_slice(&second);
        line.extend_from_slice(b"\n");
        assert!(matches!(decoder.push(&line), Err(SdkError::Stream(_))));
    }
}
