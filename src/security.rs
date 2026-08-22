use std::net::IpAddr;

use reqwest::Url;

use crate::{Result, SdkError};

pub const DEFAULT_MAX_RESPONSE_BYTES: usize = 64 * 1024 * 1024;
pub const DEFAULT_MAX_ERROR_BODY_BYTES: usize = 64 * 1024;
pub const DEFAULT_MAX_SSE_EVENT_BYTES: usize = 16 * 1024 * 1024;
pub const DEFAULT_MAX_SSE_DATA_LINES: usize = 4_096;
pub const DEFAULT_MAX_MEMORY_TEXT_BYTES: usize = 8 * 1024;
pub const DEFAULT_VECTOR_STORE_CAPACITY: usize = 10_000;

#[cfg(feature = "tools")]
pub const DEFAULT_MAX_TOOL_ARGUMENTS_BYTES: usize = 1024 * 1024;
#[cfg(feature = "tools")]
pub const DEFAULT_MAX_PENDING_TOOL_CALLS: usize = 128;

#[cfg(any(feature = "agents", feature = "rag"))]
pub const DEFAULT_MAX_TOOL_OUTPUT_BYTES: usize = 1024 * 1024;
#[cfg(any(feature = "agents", feature = "rag"))]
pub const DEFAULT_MAX_AGENT_STEPS: u32 = 64;
#[cfg(any(feature = "agents", feature = "rag"))]
pub const DEFAULT_MAX_TOOL_EXECUTIONS: usize = 64;

#[cfg(feature = "realtime")]
pub const DEFAULT_MAX_WS_MESSAGE_BYTES: usize = 8 * 1024 * 1024;
#[cfg(feature = "realtime")]
pub const DEFAULT_MAX_WS_FRAME_BYTES: usize = 2 * 1024 * 1024;
#[cfg(feature = "realtime")]
pub const DEFAULT_WS_WRITE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30);
#[cfg(feature = "realtime")]
pub const DEFAULT_CONNECTION_CLOSE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);

pub fn validate_http_url(value: &str, allow_insecure: bool) -> Result<()> {
    validate_url(value, &["http", "https"], allow_insecure, "HTTP(S) URL")
}

#[cfg(feature = "realtime")]
pub fn validate_ws_url(value: &str, allow_insecure: bool) -> Result<()> {
    validate_url(value, &["ws", "wss"], allow_insecure, "WebSocket URL")
}

fn validate_url(value: &str, schemes: &[&str], allow_insecure: bool, label: &str) -> Result<()> {
    let value = value.trim();
    let parsed = Url::parse(value).map_err(|_| {
        SdkError::Configuration(format!("{label} is not a valid absolute URL").into())
    })?;
    if !schemes.contains(&parsed.scheme()) {
        return Err(SdkError::Configuration(
            format!("{label} must use the {} scheme", schemes.join(" or ")).into(),
        ));
    }
    if parsed.host_str().is_none() {
        return Err(SdkError::Configuration(
            format!("{label} must include a host").into(),
        ));
    }
    if !parsed.username().is_empty() || parsed.password().is_some() {
        return Err(SdkError::Configuration(
            format!("{label} must not embed credentials").into(),
        ));
    }
    if matches!(parsed.scheme(), "http" | "ws") && !allow_insecure && !is_local_host(&parsed) {
        return Err(SdkError::Configuration(
            format!(
                "{label} uses a plaintext scheme; explicitly allow insecure transport for non-local hosts"
            )
            .into(),
        ));
    }
    Ok(())
}

fn is_local_host(url: &Url) -> bool {
    let Some(host) = url.host_str() else {
        return false;
    };
    if host.eq_ignore_ascii_case("localhost") {
        return true;
    }
    let Ok(ip) = host.parse::<IpAddr>() else {
        return false;
    };
    match ip {
        IpAddr::V4(value) => {
            let octets = value.octets();
            value.is_loopback()
                || value.is_private()
                || value.is_link_local()
                || value.is_broadcast()
                || value.is_unspecified()
                || octets[0] == 100 && (octets[1] & 0xc0) == 0x40
        }
        IpAddr::V6(value) => {
            value.is_loopback()
                || value.is_unique_local()
                || value.is_unicast_link_local()
                || value.is_unspecified()
        }
    }
}

pub fn truncate(value: &str, max_bytes: usize) -> String {
    if value.len() <= max_bytes {
        return value.to_owned();
    }
    let mut end = max_bytes;
    while end > 0 && !value.is_char_boundary(end) {
        end -= 1;
    }
    let mut truncated = value[..end].to_owned();
    truncated.push('…');
    truncated
}

pub fn mask_sensitive(text: &str, secrets: &[&str]) -> String {
    let mut masked = text.to_owned();
    for secret in secrets {
        if !secret.is_empty() {
            masked = masked.replace(secret, "[FILTERED]");
        }
    }
    mask_bearer(&mask_api_keys(&masked))
}

fn mask_bearer(text: &str) -> String {
    let chars: Vec<(usize, char)> = text.char_indices().collect();
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    let n = chars.len();
    while i < n {
        let rest = &text[chars[i].0..];
        let bytes = rest.as_bytes();
        let is_bearer = bytes.len() >= 6
            && bytes[..6].eq_ignore_ascii_case(b"bearer")
            && (bytes.len() == 6 || matches!(bytes[6], b':' | b' ' | b'\t' | b'\r' | b'\n'));
        if is_bearer {
            let mut j = i + 6;
            while j < n && (chars[j].1 == ':' || chars[j].1.is_whitespace()) {
                j += 1;
            }
            let token_start = j;
            while j < n && chars[j].1.is_ascii() && !chars[j].1.is_whitespace() && chars[j].1 != ','
            {
                j += 1;
            }
            let end = if token_start < n {
                chars[token_start].0
            } else {
                text.len()
            };
            out.push_str(&text[chars[i].0..end]);
            if j > token_start {
                out.push_str("[FILTERED]");
            }
            i = j;
        } else {
            out.push(chars[i].1);
            i += 1;
        }
    }
    out
}

fn mask_api_keys(text: &str) -> String {
    let chars: Vec<(usize, char)> = text.char_indices().collect();
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    let n = chars.len();
    while i < n {
        let id_len = api_key_id_len(&chars, i);
        if chars[i].1 == '.' && id_len >= 3 && api_key_secret_len(&chars, i) >= 10 {
            for _ in 0..id_len {
                out.pop();
            }
            out.push_str("[FILTERED]");
            let secret_end = i + 1 + api_key_secret_len(&chars, i);
            i = secret_end;
        } else {
            out.push(chars[i].1);
            i += 1;
        }
    }
    out
}

fn is_key_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_' || c == '-'
}

fn api_key_id_len(chars: &[(usize, char)], dot: usize) -> usize {
    let mut len = 0;
    let mut k = dot;
    while k > 0 && is_key_char(chars[k - 1].1) {
        len += 1;
        k -= 1;
    }
    len
}

fn api_key_secret_len(chars: &[(usize, char)], dot: usize) -> usize {
    let mut len = 0;
    let mut k = dot + 1;
    while k < chars.len() && is_key_char(chars[k].1) {
        len += 1;
        k += 1;
    }
    len
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn http_url_policy_rejects_unsafe_shapes() {
        for value in [
            "file:///etc/passwd",
            "ftp://example.com/x",
            "javascript:alert(1)",
            "relative/path",
            "https://user:pass@example.com/x",
            "https://",
        ] {
            assert!(validate_http_url(value, false).is_err(), "{value}");
        }
        assert!(validate_http_url("https://example.com/x", false).is_ok());
        assert!(validate_http_url("http://localhost:8080/x", false).is_ok());
        assert!(validate_http_url("http://127.0.0.1/x", false).is_ok());
        assert!(validate_http_url("http://192.168.1.10/x", false).is_ok());
        assert!(validate_http_url("http://example.com/x", false).is_err());
        assert!(validate_http_url("http://example.com/x", true).is_ok());
    }

    #[cfg(feature = "realtime")]
    #[test]
    fn ws_url_policy_requires_secure_transport_by_default() {
        assert!(validate_ws_url("ws://example.com", false).is_err());
        assert!(validate_ws_url("wss://example.com", false).is_ok());
        assert!(validate_ws_url("ws://localhost:8080", false).is_ok());
        assert!(validate_ws_url("ws://example.com", true).is_ok());
        assert!(validate_ws_url("http://example.com", false).is_err());
        assert!(validate_ws_url("wss://user:pass@example.com", false).is_err());
    }

    #[test]
    fn truncate_keeps_utf8_boundaries() {
        assert_eq!(truncate("short", 100), "short");
        assert_eq!(truncate("你好世界", 7), "你好…");
        assert_eq!(truncate("abc", 3), "abc");
    }

    #[test]
    fn mask_sensitive_redacts_credentials_and_bearer_tokens() {
        assert_eq!(
            mask_sensitive("Authorization: Bearer abc.defghijklm123", &[]),
            "Authorization: Bearer [FILTERED]"
        );
        assert_eq!(
            mask_sensitive("key=abc.defghijklm123 and more", &[]),
            "key=[FILTERED] and more"
        );
        assert_eq!(
            mask_sensitive("token abc.defghijklm", &["abc.defghijklm"]),
            "token [FILTERED]"
        );
        assert_eq!(mask_sensitive("Bearer xy", &[]), "Bearer [FILTERED]");
        assert_eq!(
            mask_sensitive("no secrets here", &["nope"]),
            "no secrets here"
        );
        assert_eq!(
            mask_sensitive("version 1.2.3 and com.example", &[]),
            "version 1.2.3 and com.example"
        );
        assert_eq!(
            mask_sensitive("前置 Bearer tok_123，中文保留", &[]),
            "前置 Bearer [FILTERED]，中文保留"
        );
    }

    #[test]
    fn mask_sensitive_matches_known_secrets_everywhere() {
        let secret = "key_id.verylongsecretvalue123";
        let text = format!("before {secret} after {secret}");
        let masked = mask_sensitive(&text, &[secret]);
        assert!(!masked.contains(secret));
        assert_eq!(masked.matches("[FILTERED]").count(), 2);
    }
}
