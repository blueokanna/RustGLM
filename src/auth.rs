use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use hmac::{Hmac, KeyInit, Mac};
use nextjson::NsonSerialize as Serialize;
use reqwest::header::HeaderValue;
use sha2::Sha256;

use crate::{Result, SdkError};

pub const DEFAULT_JWT_TTL: Duration = Duration::from_secs(180);
pub const DEFAULT_JWT_REFRESH_BEFORE: Duration = Duration::from_secs(30);

#[derive(Clone)]
pub enum ZhipuAuthentication {
    Auto(String),
    Bearer(String),
    Jwt(JwtAuthentication),
}

impl ZhipuAuthentication {
    pub fn auto(value: impl Into<String>) -> Self {
        Self::Auto(value.into())
    }

    pub fn bearer(value: impl Into<String>) -> Self {
        Self::Bearer(value.into())
    }

    pub fn jwt(value: JwtAuthentication) -> Self {
        Self::Jwt(value)
    }
}

#[derive(Clone)]
pub struct JwtAuthentication {
    api_key: String,
    api_secret: String,
    token_ttl: Duration,
    refresh_before: Duration,
    cache_enabled: bool,
}

impl JwtAuthentication {
    pub fn new(api_key: impl Into<String>, api_secret: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            api_secret: api_secret.into(),
            token_ttl: DEFAULT_JWT_TTL,
            refresh_before: DEFAULT_JWT_REFRESH_BEFORE,
            cache_enabled: true,
        }
    }

    pub fn from_api_key(value: impl AsRef<str>) -> Result<Self> {
        let value = value.as_ref().trim();
        let mut parts = value.split('.');
        let api_key = parts.next().unwrap_or_default();
        let api_secret = parts.next().unwrap_or_default();
        if api_key.is_empty() || api_secret.is_empty() || parts.next().is_some() {
            return Err(SdkError::Configuration(
                "JWT API key must contain exactly one dot separating key id and secret".into(),
            ));
        }
        if api_key.chars().any(char::is_whitespace) || api_secret.chars().any(char::is_whitespace) {
            return Err(SdkError::Configuration(
                "JWT key id and secret cannot contain whitespace".into(),
            ));
        }
        Ok(Self::new(api_key, api_secret))
    }

    pub fn token_ttl(mut self, value: Duration) -> Self {
        self.token_ttl = value;
        self
    }

    pub fn refresh_before(mut self, value: Duration) -> Self {
        self.refresh_before = value;
        self
    }

    pub fn cache_enabled(mut self, value: bool) -> Self {
        self.cache_enabled = value;
        self
    }

    pub fn api_key(&self) -> &str {
        &self.api_key
    }

    pub fn token_ttl_value(&self) -> Duration {
        self.token_ttl
    }

    pub fn refresh_before_value(&self) -> Duration {
        self.refresh_before
    }

    pub fn is_cache_enabled(&self) -> bool {
        self.cache_enabled
    }

    pub fn generate_token(&self) -> Result<String> {
        self.validate()?;
        generate_token_at(self, unix_time_millis()?)
    }

    fn validate(&self) -> Result<()> {
        if self.api_key.trim().is_empty() || self.api_secret.is_empty() {
            return Err(SdkError::Configuration(
                "JWT key id and secret cannot be empty".into(),
            ));
        }
        if self.api_key.chars().any(char::is_whitespace)
            || self.api_secret.chars().any(char::is_whitespace)
        {
            return Err(SdkError::Configuration(
                "JWT key id and secret cannot contain whitespace".into(),
            ));
        }
        if self.token_ttl.is_zero() {
            return Err(SdkError::Configuration(
                "JWT token TTL must be greater than zero".into(),
            ));
        }
        if self.refresh_before >= self.token_ttl {
            return Err(SdkError::Configuration(
                "JWT refresh window must be shorter than token TTL".into(),
            ));
        }
        Ok(())
    }
}

#[derive(Clone)]
pub(crate) struct AuthenticationProvider {
    inner: Arc<AuthenticationState>,
}

enum AuthenticationState {
    Static(String),
    Jwt {
        config: JwtAuthentication,
        cache: Mutex<Option<CachedToken>>,
    },
}

struct CachedToken {
    value: String,
    refresh_at_millis: u64,
}

impl AuthenticationProvider {
    pub(crate) fn zhipu(value: ZhipuAuthentication) -> Result<Self> {
        match value {
            ZhipuAuthentication::Auto(value) => match JwtAuthentication::from_api_key(&value) {
                Ok(config) => Self::jwt(config),
                Err(_) if !value.trim().is_empty() && !value.contains('.') => Self::bearer(value),
                Err(error) => Err(error),
            },
            ZhipuAuthentication::Bearer(value) => Self::bearer(value),
            ZhipuAuthentication::Jwt(config) => Self::jwt(config),
        }
    }

    pub(crate) fn bearer(value: impl Into<String>) -> Result<Self> {
        let value = value.into();
        if value.trim().is_empty() {
            return Err(SdkError::Configuration(
                "bearer token cannot be empty".into(),
            ));
        }
        Ok(Self {
            inner: Arc::new(AuthenticationState::Static(value)),
        })
    }

    pub(crate) fn jwt(config: JwtAuthentication) -> Result<Self> {
        config.validate()?;
        Ok(Self {
            inner: Arc::new(AuthenticationState::Jwt {
                config,
                cache: Mutex::new(None),
            }),
        })
    }

    pub(crate) fn header_value(&self) -> Result<HeaderValue> {
        let token = self.token()?;
        HeaderValue::from_str(&format!("Bearer {token}")).map_err(|_| {
            SdkError::Configuration("authentication value is not a valid header".into())
        })
    }

    fn token(&self) -> Result<String> {
        match self.inner.as_ref() {
            AuthenticationState::Static(value) => Ok(value.trim().to_owned()),
            AuthenticationState::Jwt { config, cache } => {
                let now = unix_time_millis()?;
                if config.cache_enabled {
                    let mut cache = cache.lock().map_err(|_| {
                        SdkError::Configuration("JWT cache lock is poisoned".into())
                    })?;
                    if let Some(token) =
                        cache.as_ref().filter(|token| now < token.refresh_at_millis)
                    {
                        return Ok(token.value.clone());
                    }
                    let value = generate_token_at(config, now)?;
                    let refresh_at_millis = now
                        .checked_add(duration_millis(config.token_ttl)?)
                        .and_then(|value| {
                            value.checked_sub(duration_millis(config.refresh_before).ok()?)
                        })
                        .ok_or_else(|| {
                            SdkError::Configuration("JWT refresh time overflow".into())
                        })?;
                    *cache = Some(CachedToken {
                        value: value.clone(),
                        refresh_at_millis,
                    });
                    Ok(value)
                } else {
                    generate_token_at(config, now)
                }
            }
        }
    }
}

#[derive(Serialize)]
struct JwtHeader<'a> {
    alg: &'a str,
    sign_type: &'a str,
}

#[derive(Serialize)]
struct JwtPayload<'a> {
    api_key: &'a str,
    exp: u64,
    timestamp: u64,
}

fn generate_token_at(config: &JwtAuthentication, now_millis: u64) -> Result<String> {
    config.validate()?;
    let exp = now_millis
        .checked_add(duration_millis(config.token_ttl)?)
        .ok_or_else(|| SdkError::Configuration("JWT expiration time overflow".into()))?;
    let header = encode_json(&JwtHeader {
        alg: "HS256",
        sign_type: "SIGN",
    })?;
    let payload = encode_json(&JwtPayload {
        api_key: &config.api_key,
        exp,
        timestamp: now_millis,
    })?;
    let signing_input = format!("{header}.{payload}");
    let mut mac = Hmac::<Sha256>::new_from_slice(config.api_secret.as_bytes())
        .map_err(|_| SdkError::Configuration("JWT secret is invalid".into()))?;
    mac.update(signing_input.as_bytes());
    let signature = URL_SAFE_NO_PAD.encode(mac.finalize().into_bytes());
    Ok(format!("{signing_input}.{signature}"))
}

fn encode_json<T: Serialize>(value: &T) -> Result<String> {
    let bytes = nextjson::to_vec(value)
        .map_err(|error| SdkError::Configuration(error.to_string().into()))?;
    Ok(URL_SAFE_NO_PAD.encode(bytes))
}

fn unix_time_millis() -> Result<u64> {
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| SdkError::Configuration("system clock is before Unix epoch".into()))?;
    duration_millis(duration)
}

fn duration_millis(value: Duration) -> Result<u64> {
    u64::try_from(value.as_millis())
        .map_err(|_| SdkError::Configuration("duration exceeds supported range".into()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use nextjson::Value;

    fn decode_segment(value: &str) -> Value {
        let bytes = URL_SAFE_NO_PAD.decode(value).unwrap();
        nextjson::from_slice(&bytes).unwrap()
    }

    #[test]
    fn parses_combined_api_key() {
        let config = JwtAuthentication::from_api_key("key.secret").unwrap();
        assert_eq!(config.api_key(), "key");
        assert_eq!(config.token_ttl_value(), Duration::from_secs(180));
        assert_eq!(config.refresh_before_value(), Duration::from_secs(30));
    }

    #[test]
    fn rejects_invalid_combined_api_keys() {
        for value in [
            "",
            "key",
            ".secret",
            "key.",
            "a.b.c",
            "key. secret",
            "key .secret",
        ] {
            assert!(JwtAuthentication::from_api_key(value).is_err(), "{value}");
        }
    }

    #[test]
    fn generates_official_jwt_shape() {
        let config = JwtAuthentication::new("key", "secret");
        let token = generate_token_at(&config, 1_700_000_000_000).unwrap();
        let parts: Vec<_> = token.split('.').collect();
        assert_eq!(parts.len(), 3);
        assert_eq!(
            decode_segment(parts[0]),
            nextjson::json!({"alg":"HS256","sign_type":"SIGN"})
        );
        assert_eq!(
            decode_segment(parts[1]),
            nextjson::json!({"api_key":"key","exp":1700000180000u64,"timestamp":1700000000000u64})
        );
        assert!(!parts[2].contains('='));
    }

    #[test]
    fn signature_is_hs256_and_verifiable() {
        let config = JwtAuthentication::new("key", "secret");
        let token = generate_token_at(&config, 42).unwrap();
        let parts: Vec<_> = token.split('.').collect();
        let mut mac = Hmac::<Sha256>::new_from_slice(b"secret").unwrap();
        mac.update(format!("{}.{}", parts[0], parts[1]).as_bytes());
        let signature = URL_SAFE_NO_PAD.decode(parts[2]).unwrap();
        mac.verify_slice(&signature).unwrap();
    }

    #[test]
    fn validates_ttl_and_refresh_window() {
        assert!(
            JwtAuthentication::new("key", "secret")
                .token_ttl(Duration::ZERO)
                .generate_token()
                .is_err()
        );
        assert!(
            JwtAuthentication::new("key", "secret")
                .token_ttl(Duration::from_secs(10))
                .refresh_before(Duration::from_secs(10))
                .generate_token()
                .is_err()
        );
    }

    #[test]
    fn auto_selects_jwt_for_combined_key() {
        let provider =
            AuthenticationProvider::zhipu(ZhipuAuthentication::auto("key.secret")).unwrap();
        let header = provider.header_value().unwrap();
        assert_eq!(header.to_str().unwrap().matches('.').count(), 2);
    }

    #[test]
    fn auto_keeps_non_combined_key_as_bearer() {
        let provider =
            AuthenticationProvider::zhipu(ZhipuAuthentication::auto("opaque-key")).unwrap();
        assert_eq!(provider.header_value().unwrap(), "Bearer opaque-key");
    }

    #[test]
    fn cached_provider_reuses_token() {
        let provider =
            AuthenticationProvider::jwt(JwtAuthentication::new("key", "secret")).unwrap();
        assert_eq!(provider.token().unwrap(), provider.token().unwrap());
    }

    #[test]
    fn disabled_cache_still_generates_valid_token() {
        let provider = AuthenticationProvider::jwt(
            JwtAuthentication::new("key", "secret").cache_enabled(false),
        )
        .unwrap();
        assert_eq!(provider.token().unwrap().split('.').count(), 3);
    }

    #[test]
    fn explicit_authentication_constructors_preserve_modes() {
        assert!(matches!(
            ZhipuAuthentication::auto("value"),
            ZhipuAuthentication::Auto(_)
        ));
        assert!(matches!(
            ZhipuAuthentication::bearer("value"),
            ZhipuAuthentication::Bearer(_)
        ));
        assert!(matches!(
            ZhipuAuthentication::jwt(JwtAuthentication::new("key", "secret")),
            ZhipuAuthentication::Jwt(_)
        ));
    }

    #[test]
    fn explicit_bearer_validates_empty_and_invalid_headers() {
        assert!(AuthenticationProvider::bearer("").is_err());
        let provider = AuthenticationProvider::bearer("invalid\nvalue").unwrap();
        assert!(provider.header_value().is_err());
    }

    #[test]
    fn custom_jwt_settings_are_observable() {
        let config = JwtAuthentication::new("key", "secret")
            .token_ttl(Duration::from_secs(60))
            .refresh_before(Duration::from_secs(5))
            .cache_enabled(false);
        assert_eq!(config.token_ttl_value(), Duration::from_secs(60));
        assert_eq!(config.refresh_before_value(), Duration::from_secs(5));
        assert!(!config.is_cache_enabled());
    }

    #[test]
    fn explicit_jwt_rejects_empty_credentials() {
        assert!(AuthenticationProvider::jwt(JwtAuthentication::new("", "secret")).is_err());
        assert!(AuthenticationProvider::jwt(JwtAuthentication::new("key", "")).is_err());
    }
}
