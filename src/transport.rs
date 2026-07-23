use std::time::Duration;

use reqwest::header::{ACCEPT, AUTHORIZATION, CONTENT_TYPE, HeaderMap, HeaderValue, USER_AGENT};
use reqwest::{Client, Method, Response, StatusCode};
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::Value;
use tokio::time::sleep;

use crate::auth::AuthenticationProvider;
use crate::{ApiError, Result, SdkError};

#[derive(Debug, Clone)]
pub struct RetryPolicy {
    pub max_retries: u32,
    pub initial_delay: Duration,
    pub max_delay: Duration,
    pub retry_statuses: Vec<StatusCode>,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_retries: 0,
            initial_delay: Duration::from_millis(250),
            max_delay: Duration::from_secs(4),
            retry_statuses: vec![
                StatusCode::REQUEST_TIMEOUT,
                StatusCode::TOO_MANY_REQUESTS,
                StatusCode::INTERNAL_SERVER_ERROR,
                StatusCode::BAD_GATEWAY,
                StatusCode::SERVICE_UNAVAILABLE,
                StatusCode::GATEWAY_TIMEOUT,
            ],
        }
    }
}

#[derive(Clone)]
pub struct HttpConfig {
    pub timeout: Duration,
    pub connect_timeout: Duration,
    pub pool_idle_timeout: Duration,
    pub user_agent: String,
    pub default_headers: HeaderMap,
    pub retry: RetryPolicy,
    pub http_client: Option<Client>,
}

impl Default for HttpConfig {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(120),
            connect_timeout: Duration::from_secs(10),
            pool_idle_timeout: Duration::from_secs(90),
            user_agent: format!("RustGLM/{}", env!("CARGO_PKG_VERSION")),
            default_headers: HeaderMap::new(),
            retry: RetryPolicy::default(),
            http_client: None,
        }
    }
}

#[derive(Clone)]
pub(crate) struct Transport {
    client: Client,
    base_url: String,
    authentication: AuthenticationProvider,
    headers: HeaderMap,
    retry: RetryPolicy,
}

impl Transport {
    pub(crate) fn new(
        base_url: String,
        authentication: AuthenticationProvider,
        config: HttpConfig,
    ) -> Result<Self> {
        let base_url = normalize_base_url(base_url)?;
        let mut headers = config.default_headers;
        if !headers.contains_key(USER_AGENT) {
            headers.insert(
                USER_AGENT,
                HeaderValue::from_str(&config.user_agent)
                    .map_err(|_| SdkError::Configuration("user agent is invalid".into()))?,
            );
        }
        let client = match config.http_client {
            Some(client) => client,
            None => Client::builder()
                .timeout(config.timeout)
                .connect_timeout(config.connect_timeout)
                .pool_idle_timeout(config.pool_idle_timeout)
                .build()?,
        };
        Ok(Self {
            client,
            base_url,
            authentication,
            headers,
            retry: config.retry,
        })
    }

    pub(crate) fn base_url(&self) -> &str {
        &self.base_url
    }

    pub(crate) async fn post_json<T, R>(&self, path: &str, body: &T) -> Result<R>
    where
        T: Serialize + ?Sized,
        R: DeserializeOwned,
    {
        let bytes = serde_json::to_vec(body)
            .map_err(|error| SdkError::Validation(error.to_string().into()))?;
        let response = self
            .send_bytes(
                Method::POST,
                path,
                bytes,
                "application/json",
                "application/json",
            )
            .await?;
        self.decode_json(response).await
    }

    pub(crate) async fn post_stream<T: Serialize + ?Sized>(
        &self,
        path: &str,
        body: &T,
    ) -> Result<Response> {
        let bytes = serde_json::to_vec(body)
            .map_err(|error| SdkError::Validation(error.to_string().into()))?;
        self.send_bytes(
            Method::POST,
            path,
            bytes,
            "application/json",
            "text/event-stream",
        )
        .await
    }

    #[cfg_attr(not(feature = "rag"), allow(dead_code))]
    pub(crate) async fn post_stream_with_headers<T: Serialize + ?Sized>(
        &self,
        path: &str,
        body: &T,
        headers: HeaderMap,
    ) -> Result<Response> {
        let bytes = serde_json::to_vec(body)
            .map_err(|error| SdkError::Validation(error.to_string().into()))?;
        self.send_bytes_with_headers(
            Method::POST,
            path,
            bytes,
            "application/json",
            "text/event-stream",
            headers,
        )
        .await
    }

    #[cfg_attr(not(feature = "audio"), allow(dead_code))]
    pub(crate) async fn post_binary<T: Serialize + ?Sized>(
        &self,
        path: &str,
        body: &T,
        accept: &str,
    ) -> Result<Vec<u8>> {
        let bytes = serde_json::to_vec(body)
            .map_err(|error| SdkError::Validation(error.to_string().into()))?;
        let response = self
            .send_bytes(Method::POST, path, bytes, "application/json", accept)
            .await?;
        Ok(response.bytes().await?.to_vec())
    }

    pub(crate) async fn get_json<R: DeserializeOwned>(&self, path: &str) -> Result<R> {
        let response = self
            .send_empty(Method::GET, path, "application/json")
            .await?;
        self.decode_json(response).await
    }

    #[cfg_attr(not(feature = "files"), allow(dead_code))]
    pub(crate) async fn get_binary(&self, path: &str) -> Result<Vec<u8>> {
        let response = self.send_empty(Method::GET, path, "*/*").await?;
        Ok(response.bytes().await?.to_vec())
    }

    #[cfg_attr(not(any(feature = "files", feature = "rag")), allow(dead_code))]
    pub(crate) async fn delete_json<R: DeserializeOwned>(&self, path: &str) -> Result<R> {
        let response = self
            .send_empty(Method::DELETE, path, "application/json")
            .await?;
        self.decode_json(response).await
    }

    #[cfg_attr(
        not(any(feature = "audio", feature = "files", feature = "rag")),
        allow(dead_code)
    )]
    pub(crate) async fn post_multipart<R: DeserializeOwned>(
        &self,
        path: &str,
        form: reqwest::multipart::Form,
    ) -> Result<R> {
        let url = self.url(path)?;
        let response = self
            .client
            .post(url)
            .headers(self.headers.clone())
            .header(AUTHORIZATION, self.authentication.header_value()?)
            .header(ACCEPT, "application/json")
            .multipart(form)
            .send()
            .await?;
        let response = self.ensure_success(response).await?;
        self.decode_json(response).await
    }

    pub(crate) async fn request_json<T, R>(
        &self,
        method: Method,
        path: &str,
        body: Option<&T>,
    ) -> Result<R>
    where
        T: Serialize + ?Sized,
        R: DeserializeOwned,
    {
        let response = match body {
            Some(body) => {
                let bytes = serde_json::to_vec(body)
                    .map_err(|error| SdkError::Validation(error.to_string().into()))?;
                self.send_bytes(method, path, bytes, "application/json", "application/json")
                    .await?
            }
            None => self.send_empty(method, path, "application/json").await?,
        };
        self.decode_json(response).await
    }

    async fn send_bytes(
        &self,
        method: Method,
        path: &str,
        body: Vec<u8>,
        content_type: &str,
        accept: &str,
    ) -> Result<Response> {
        self.send_bytes_with_headers(method, path, body, content_type, accept, HeaderMap::new())
            .await
    }

    async fn send_bytes_with_headers(
        &self,
        method: Method,
        path: &str,
        body: Vec<u8>,
        content_type: &str,
        accept: &str,
        extra_headers: HeaderMap,
    ) -> Result<Response> {
        let url = self.url(path)?;
        let mut attempt = 0;
        loop {
            let response = self
                .client
                .request(method.clone(), &url)
                .headers(self.headers.clone())
                .headers(extra_headers.clone())
                .header(AUTHORIZATION, self.authentication.header_value()?)
                .header(CONTENT_TYPE, content_type)
                .header(ACCEPT, accept)
                .body(body.clone())
                .send()
                .await;
            match response {
                Ok(response)
                    if self.retry.retry_statuses.contains(&response.status())
                        && attempt < self.retry.max_retries =>
                {
                    let delay = retry_delay(&response, &self.retry, attempt);
                    drop(response);
                    sleep(delay).await;
                    attempt += 1;
                }
                Ok(response) => return self.ensure_success(response).await,
                Err(error)
                    if (error.is_connect() || error.is_timeout())
                        && attempt < self.retry.max_retries =>
                {
                    sleep(backoff(&self.retry, attempt)).await;
                    attempt += 1;
                }
                Err(error) => return Err(error.into()),
            }
        }
    }

    async fn send_empty(&self, method: Method, path: &str, accept: &str) -> Result<Response> {
        let url = self.url(path)?;
        let mut attempt = 0;
        loop {
            let response = self
                .client
                .request(method.clone(), &url)
                .headers(self.headers.clone())
                .header(AUTHORIZATION, self.authentication.header_value()?)
                .header(ACCEPT, accept)
                .send()
                .await;
            match response {
                Ok(response)
                    if self.retry.retry_statuses.contains(&response.status())
                        && attempt < self.retry.max_retries =>
                {
                    let delay = retry_delay(&response, &self.retry, attempt);
                    drop(response);
                    sleep(delay).await;
                    attempt += 1;
                }
                Ok(response) => return self.ensure_success(response).await,
                Err(error)
                    if (error.is_connect() || error.is_timeout())
                        && attempt < self.retry.max_retries =>
                {
                    sleep(backoff(&self.retry, attempt)).await;
                    attempt += 1;
                }
                Err(error) => return Err(error.into()),
            }
        }
    }

    async fn ensure_success(&self, response: Response) -> Result<Response> {
        if response.status().is_success() {
            return Ok(response);
        }
        let status = response.status();
        let request_id = response
            .headers()
            .get("x-request-id")
            .or_else(|| response.headers().get("x-zhipu-request-id"))
            .and_then(|value| value.to_str().ok())
            .map(str::to_owned);
        let body = response.text().await.unwrap_or_default();
        let value = serde_json::from_str::<Value>(&body).ok();
        let error = value
            .as_ref()
            .and_then(|value| value.get("error"))
            .unwrap_or_else(|| value.as_ref().unwrap_or(&Value::Null));
        let code = error.get("code").and_then(value_to_string).or_else(|| {
            value
                .as_ref()
                .and_then(|value| value.get("code"))
                .and_then(value_to_string)
        });
        let message = error
            .get("message")
            .and_then(Value::as_str)
            .or_else(|| {
                value
                    .as_ref()
                    .and_then(|value| value.get("message"))
                    .and_then(Value::as_str)
            })
            .unwrap_or(&body)
            .to_owned();
        Err(SdkError::Api(ApiError {
            status,
            code,
            message,
            request_id,
            body,
        }))
    }

    async fn decode_json<R: DeserializeOwned>(&self, response: Response) -> Result<R> {
        let body = response.bytes().await?;
        serde_json::from_slice(&body).map_err(|error| SdkError::Decode {
            message: error.to_string(),
            body: String::from_utf8_lossy(&body).into_owned(),
        })
    }

    fn url(&self, path: &str) -> Result<String> {
        validate_path(path)?;
        Ok(format!(
            "{}/{}",
            self.base_url,
            path.trim_start_matches('/')
        ))
    }

    #[cfg(test)]
    #[allow(dead_code)]
    pub(crate) fn url_for_test(&self, path: &str) -> Result<String> {
        self.url(path)
    }
}

fn normalize_base_url(value: String) -> Result<String> {
    let value = value.trim().trim_end_matches('/');
    if !(value.starts_with("https://") || value.starts_with("http://")) {
        return Err(SdkError::Configuration(
            "base URL must use http or https".into(),
        ));
    }
    Ok(value.to_owned())
}

fn validate_path(path: &str) -> Result<()> {
    if path.trim().is_empty()
        || path.starts_with("http://")
        || path.starts_with("https://")
        || path.split(['/', '?']).any(|part| part == "..")
    {
        return Err(SdkError::Validation(
            "API path must be a safe relative path".into(),
        ));
    }
    Ok(())
}

fn value_to_string(value: &Value) -> Option<String> {
    match value {
        Value::String(value) => Some(value.clone()),
        Value::Number(value) => Some(value.to_string()),
        _ => None,
    }
}

fn retry_delay(response: &Response, retry: &RetryPolicy, attempt: u32) -> Duration {
    response
        .headers()
        .get("retry-after")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<u64>().ok())
        .map(Duration::from_secs)
        .unwrap_or_else(|| backoff(retry, attempt))
        .min(retry.max_delay)
}

fn backoff(retry: &RetryPolicy, attempt: u32) -> Duration {
    retry
        .initial_delay
        .saturating_mul(2u32.saturating_pow(attempt))
        .min(retry.max_delay)
}

#[cfg(test)]
mod tests {
    use serde_json::{Value, json};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    use super::*;

    struct MockResponse {
        status: &'static str,
        headers: &'static str,
        body: &'static str,
    }

    async fn mock_server(
        responses: Vec<MockResponse>,
    ) -> (String, tokio::task::JoinHandle<Vec<String>>) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let mut requests = Vec::new();
            for response in responses {
                let (mut socket, _) = listener.accept().await.unwrap();
                let mut request = Vec::new();
                let mut buffer = [0u8; 4096];
                let mut expected = None;
                loop {
                    let read = socket.read(&mut buffer).await.unwrap();
                    if read == 0 {
                        break;
                    }
                    request.extend_from_slice(&buffer[..read]);
                    if let Some(end) = request
                        .windows(4)
                        .position(|part| part == b"\r\n\r\n")
                        .filter(|_| expected.is_none())
                    {
                        let headers = String::from_utf8_lossy(&request[..end]);
                        let content_length = headers
                            .lines()
                            .find_map(|line| {
                                let (name, value) = line.split_once(':')?;
                                name.eq_ignore_ascii_case("content-length")
                                    .then(|| value.trim().parse::<usize>().ok())
                                    .flatten()
                            })
                            .unwrap_or(0);
                        expected = Some(end + 4 + content_length);
                    }
                    if expected.is_some_and(|length| request.len() >= length) {
                        break;
                    }
                }
                requests.push(String::from_utf8_lossy(&request).into_owned());
                let output = format!(
                    "HTTP/1.1 {}\r\n{}Content-Length: {}\r\nConnection: close\r\n\r\n{}",
                    response.status,
                    response.headers,
                    response.body.len(),
                    response.body
                );
                socket.write_all(output.as_bytes()).await.unwrap();
            }
            requests
        });
        (format!("http://{address}"), server)
    }

    fn transport(base_url: String, retry: RetryPolicy) -> Transport {
        Transport::new(
            base_url,
            AuthenticationProvider::bearer("secret").unwrap(),
            HttpConfig {
                retry,
                ..HttpConfig::default()
            },
        )
        .unwrap()
    }

    #[test]
    fn validates_configuration_paths_and_backoff() {
        assert!(normalize_base_url("ftp://example.com".into()).is_err());
        assert_eq!(
            normalize_base_url(" https://example.com/// ".into()).unwrap(),
            "https://example.com"
        );
        for path in [
            "",
            " ",
            "http://other",
            "https://other",
            "../x",
            "a/../x",
            "a?../x",
        ] {
            assert!(validate_path(path).is_err(), "{path}");
        }
        assert!(validate_path("models/a?x=1").is_ok());
        assert_eq!(value_to_string(&json!(42)).as_deref(), Some("42"));
        assert_eq!(value_to_string(&json!("code")).as_deref(), Some("code"));
        assert_eq!(value_to_string(&json!(true)), None);
        let retry = RetryPolicy {
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_millis(250),
            ..RetryPolicy::default()
        };
        assert_eq!(backoff(&retry, 0), Duration::from_millis(100));
        assert_eq!(backoff(&retry, 1), Duration::from_millis(200));
        assert_eq!(backoff(&retry, 40), Duration::from_millis(250));
        let http = HttpConfig {
            user_agent: "bad\nagent".into(),
            ..HttpConfig::default()
        };
        assert!(
            Transport::new(
                "https://example.com".into(),
                AuthenticationProvider::bearer("key").unwrap(),
                http
            )
            .is_err()
        );
    }

    #[tokio::test]
    async fn decodes_nested_root_and_invalid_api_responses() {
        let (base_url, server) = mock_server(vec![
            MockResponse {
                status: "400 Bad Request",
                headers: "Content-Type: application/json\r\nX-Request-Id: req-1\r\n",
                body: r#"{"error":{"code":1210,"message":"nested"}}"#,
            },
            MockResponse {
                status: "401 Unauthorized",
                headers: "Content-Type: application/json\r\nX-Zhipu-Request-Id: req-2\r\n",
                body: r#"{"code":"unauthorized","message":"root"}"#,
            },
            MockResponse {
                status: "500 Internal Server Error",
                headers: "Content-Type: text/plain\r\n",
                body: "plain failure",
            },
            MockResponse {
                status: "200 OK",
                headers: "Content-Type: application/json\r\n",
                body: "not-json",
            },
        ])
        .await;
        let client = transport(base_url, RetryPolicy::default());
        let first = client
            .post_json::<_, Value>("one", &json!({}))
            .await
            .unwrap_err();
        match first {
            SdkError::Api(error) => {
                assert_eq!(error.status, StatusCode::BAD_REQUEST);
                assert_eq!(error.code.as_deref(), Some("1210"));
                assert_eq!(error.message, "nested");
                assert_eq!(error.request_id.as_deref(), Some("req-1"));
                assert!(error.body.contains("nested"));
            }
            _ => panic!("expected API error"),
        }
        let second = client.get_json::<Value>("two").await.unwrap_err();
        match second {
            SdkError::Api(error) => {
                assert_eq!(error.code.as_deref(), Some("unauthorized"));
                assert_eq!(error.message, "root");
                assert_eq!(error.request_id.as_deref(), Some("req-2"));
            }
            _ => panic!("expected API error"),
        }
        let third = client.get_json::<Value>("three").await.unwrap_err();
        match third {
            SdkError::Api(error) => assert_eq!(error.message, "plain failure"),
            _ => panic!("expected API error"),
        }
        let fourth = client.get_json::<Value>("four").await.unwrap_err();
        match fourth {
            SdkError::Decode { body, .. } => assert_eq!(body, "not-json"),
            _ => panic!("expected decode error"),
        }
        assert_eq!(server.await.unwrap().len(), 4);
    }

    #[tokio::test]
    async fn retries_statuses_for_body_and_empty_requests() {
        let (base_url, server) = mock_server(vec![
            MockResponse {
                status: "429 Too Many Requests",
                headers: "Retry-After: 0\r\n",
                body: "retry",
            },
            MockResponse {
                status: "200 OK",
                headers: "Content-Type: application/json\r\n",
                body: r#"{"ok":true}"#,
            },
            MockResponse {
                status: "503 Service Unavailable",
                headers: "Retry-After: invalid\r\n",
                body: "retry",
            },
            MockResponse {
                status: "200 OK",
                headers: "Content-Type: application/json\r\n",
                body: r#"{"ok":true}"#,
            },
        ])
        .await;
        let client = transport(
            base_url,
            RetryPolicy {
                max_retries: 1,
                initial_delay: Duration::ZERO,
                max_delay: Duration::ZERO,
                ..RetryPolicy::default()
            },
        );
        let post: Value = client.post_json("post", &json!({"value":1})).await.unwrap();
        let get: Value = client.get_json("get").await.unwrap();
        assert_eq!(post["ok"], true);
        assert_eq!(get["ok"], true);
        let requests = server.await.unwrap();
        assert_eq!(requests.len(), 4);
        assert!(requests[0].starts_with("POST /post "));
        assert!(requests[1].starts_with("POST /post "));
        assert!(requests[2].starts_with("GET /get "));
        assert!(requests[3].starts_with("GET /get "));
    }

    #[tokio::test]
    async fn sends_binary_delete_and_custom_client_requests() {
        let (base_url, server) = mock_server(vec![
            MockResponse {
                status: "200 OK",
                headers: "Content-Type: application/octet-stream\r\n",
                body: "post-bytes",
            },
            MockResponse {
                status: "200 OK",
                headers: "Content-Type: application/octet-stream\r\n",
                body: "get-bytes",
            },
            MockResponse {
                status: "200 OK",
                headers: "Content-Type: application/json\r\n",
                body: r#"{"deleted":true}"#,
            },
        ])
        .await;
        let custom_client = Client::builder().build().unwrap();
        let client = Transport::new(
            base_url,
            AuthenticationProvider::bearer("secret").unwrap(),
            HttpConfig {
                http_client: Some(custom_client),
                ..HttpConfig::default()
            },
        )
        .unwrap();
        assert_eq!(
            client
                .post_binary("binary", &json!({}), "audio/*")
                .await
                .unwrap(),
            b"post-bytes"
        );
        assert_eq!(client.get_binary("binary").await.unwrap(), b"get-bytes");
        let deleted: Value = client.delete_json("item").await.unwrap();
        assert_eq!(deleted["deleted"], true);
        let requests = server.await.unwrap();
        assert!(requests[0].to_ascii_lowercase().contains("accept: audio/*"));
        assert!(requests[1].to_ascii_lowercase().contains("accept: */*"));
        assert!(requests[2].starts_with("DELETE /item "));
    }
}
