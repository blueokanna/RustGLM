use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;

use nextjson::Map;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use rmcp::model::{
    CallToolRequestParams, CallToolResult, GetPromptRequestParams, GetPromptResult, Prompt,
    ReadResourceRequestParams, ReadResourceResult, Resource, ResourceTemplate, Tool,
};
use rmcp::service::RunningService;
use rmcp::transport::StreamableHttpClientTransport;
use rmcp::transport::common::client_side_sse::NeverRetry;
use rmcp::transport::streamable_http_client::StreamableHttpClientTransportConfig;
use rmcp::{Peer, RoleClient, ServiceExt};

use crate::{McpClientError, Result};

pub type McpToolDefinition = Tool;
pub type McpResource = Resource;
pub type McpResourceTemplate = ResourceTemplate;
pub type McpPrompt = Prompt;
pub type McpToolResult = CallToolResult;
pub type McpReadResourceResult = ReadResourceResult;
pub type McpGetPromptResult = GetPromptResult;

#[derive(Clone)]
pub struct McpClientConfig {
    pub endpoint: String,
    pub bearer_token: Option<String>,
    pub headers: HeaderMap,
    pub http_client: Option<reqwest::Client>,
    pub reinitialize_expired_session: bool,
}

impl fmt::Debug for McpClientConfig {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("McpClientConfig")
            .field("endpoint", &self.endpoint)
            .field(
                "bearer_token",
                &self.bearer_token.as_ref().map(|_| "[REDACTED]"),
            )
            .field("headers", &self.headers)
            .field(
                "http_client",
                &self.http_client.as_ref().map(|_| "configured"),
            )
            .field(
                "reinitialize_expired_session",
                &self.reinitialize_expired_session,
            )
            .finish()
    }
}

impl McpClientConfig {
    pub fn new(endpoint: impl Into<String>) -> Self {
        Self {
            endpoint: endpoint.into(),
            bearer_token: None,
            headers: HeaderMap::new(),
            http_client: None,
            reinitialize_expired_session: false,
        }
    }

    pub fn bearer_token(mut self, value: impl Into<String>) -> Self {
        self.bearer_token = Some(value.into());
        self
    }

    pub fn header(mut self, name: impl AsRef<str>, value: impl AsRef<str>) -> Result<Self> {
        let name = HeaderName::from_bytes(name.as_ref().as_bytes())
            .map_err(|_| McpClientError::InvalidHeader(name.as_ref().to_owned()))?;
        let value = HeaderValue::from_str(value.as_ref())
            .map_err(|_| McpClientError::InvalidHeader(name.to_string()))?;
        self.headers.insert(name, value);
        Ok(self)
    }

    pub fn headers(mut self, value: HeaderMap) -> Self {
        self.headers = value;
        self
    }

    pub fn http_client(mut self, value: reqwest::Client) -> Self {
        self.http_client = Some(value);
        self
    }

    pub fn reinitialize_expired_session(mut self, value: bool) -> Self {
        self.reinitialize_expired_session = value;
        self
    }

    pub async fn connect(self) -> Result<McpClient> {
        McpClient::connect(self).await
    }
}

pub struct McpClient {
    service: RunningService<RoleClient, ()>,
}

impl fmt::Debug for McpClient {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("McpClient")
            .field("closed", &self.service.is_closed())
            .finish()
    }
}

impl McpClient {
    pub async fn connect(config: McpClientConfig) -> Result<Self> {
        validate_endpoint(&config.endpoint)?;
        if config
            .bearer_token
            .as_ref()
            .is_some_and(|token| token.trim().is_empty())
        {
            return Err(McpClientError::InvalidHeader("authorization".into()).into());
        }

        let client = match config.http_client {
            Some(client) => client,
            None => reqwest::Client::builder()
                .redirect(reqwest::redirect::Policy::none())
                .build()
                .map_err(|error| McpClientError::ClientBuild(error.to_string()))?,
        };

        let mut transport_config = StreamableHttpClientTransportConfig::with_uri(config.endpoint);
        transport_config.retry_config = Arc::new(NeverRetry::default());
        transport_config.reinit_on_expired_session = config.reinitialize_expired_session;
        transport_config.auth_header = config.bearer_token;
        transport_config.custom_headers = config
            .headers
            .iter()
            .map(|(name, value)| (name.clone(), value.clone()))
            .collect::<HashMap<_, _>>();

        let transport = StreamableHttpClientTransport::with_client(client, transport_config);
        let service = ()
            .serve(transport)
            .await
            .map_err(|error| McpClientError::Initialize(error.to_string()))?;
        Ok(Self { service })
    }

    pub fn peer(&self) -> &Peer<RoleClient> {
        self.service.peer()
    }

    pub fn is_closed(&self) -> bool {
        self.service.is_closed()
    }

    pub async fn list_tools(&self) -> Result<Vec<McpToolDefinition>> {
        self.service
            .list_all_tools()
            .await
            .map_err(|error| McpClientError::Request(error.to_string()).into())
    }

    pub async fn call_tool(
        &self,
        name: impl Into<String>,
        arguments: Option<Map>,
    ) -> Result<McpToolResult> {
        let name = name.into();
        if name.trim().is_empty() {
            return Err(McpClientError::Request("tool name cannot be empty".into()).into());
        }
        let mut request = CallToolRequestParams::new(name);
        if let Some(arguments) = arguments {
            request = request.with_arguments(to_serde_map(&arguments));
        }
        self.service
            .call_tool(request)
            .await
            .map_err(|error| McpClientError::Request(error.to_string()).into())
    }

    pub async fn list_resources(&self) -> Result<Vec<McpResource>> {
        self.service
            .list_all_resources()
            .await
            .map_err(|error| McpClientError::Request(error.to_string()).into())
    }

    pub async fn list_resource_templates(&self) -> Result<Vec<McpResourceTemplate>> {
        self.service
            .list_all_resource_templates()
            .await
            .map_err(|error| McpClientError::Request(error.to_string()).into())
    }

    pub async fn read_resource(&self, uri: impl Into<String>) -> Result<McpReadResourceResult> {
        self.service
            .read_resource(ReadResourceRequestParams::new(uri))
            .await
            .map_err(|error| McpClientError::Request(error.to_string()).into())
    }

    pub async fn list_prompts(&self) -> Result<Vec<McpPrompt>> {
        self.service
            .list_all_prompts()
            .await
            .map_err(|error| McpClientError::Request(error.to_string()).into())
    }

    pub async fn get_prompt(
        &self,
        name: impl Into<String>,
        arguments: Option<Map>,
    ) -> Result<McpGetPromptResult> {
        let mut request = GetPromptRequestParams::default();
        request.name = name.into();
        request.arguments = arguments.as_ref().map(to_serde_map);
        self.service
            .get_prompt(request)
            .await
            .map_err(|error| McpClientError::Request(error.to_string()).into())
    }

    pub async fn close(&mut self) -> Result<()> {
        self.service
            .close()
            .await
            .map_err(|error| McpClientError::Shutdown(error.to_string()))?;
        Ok(())
    }
}

fn validate_endpoint(endpoint: &str) -> Result<()> {
    let url = reqwest::Url::parse(endpoint)
        .map_err(|error| McpClientError::InvalidEndpoint(error.to_string()))?;
    if !matches!(url.scheme(), "http" | "https") || url.host_str().is_none() {
        return Err(McpClientError::InvalidEndpoint(
            "expected an absolute http or https URL".into(),
        )
        .into());
    }
    Ok(())
}

/// Converts a nextjson map into the `serde_json` map required by the rmcp wire
/// model. This is the only serde_json contact point in the SDK: rmcp is
/// serde-native, so the conversion is isolated here and reaches serde_json
/// exclusively through rmcp's re-export.
fn to_serde_map(map: &Map) -> rmcp::serde_json::Map<String, rmcp::serde_json::Value> {
    map.iter()
        .map(|(key, value)| (key.to_owned(), to_serde_value(value)))
        .collect()
}

fn to_serde_value(value: &nextjson::Value) -> rmcp::serde_json::Value {
    match value {
        nextjson::Value::Null => rmcp::serde_json::Value::Null,
        nextjson::Value::Bool(value) => rmcp::serde_json::Value::Bool(*value),
        nextjson::Value::Number(value) => {
            if let Some(integer) = value.as_i64() {
                rmcp::serde_json::Value::Number(rmcp::serde_json::Number::from(integer))
            } else if let Some(integer) = value.as_u64() {
                rmcp::serde_json::Value::Number(rmcp::serde_json::Number::from(integer))
            } else {
                rmcp::serde_json::Number::from_f64(value.as_f64())
                    .map(rmcp::serde_json::Value::Number)
                    .unwrap_or(rmcp::serde_json::Value::Null)
            }
        }
        nextjson::Value::String(value) => rmcp::serde_json::Value::String(value.clone()),
        nextjson::Value::Array(values) => {
            rmcp::serde_json::Value::Array(values.iter().map(to_serde_value).collect())
        }
        nextjson::Value::Object(object) => rmcp::serde_json::Value::Object(to_serde_map(object)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn configuration_is_explicit_and_redacts_tokens() {
        let mut headers = HeaderMap::new();
        headers.insert("x-source", HeaderValue::from_static("test"));
        let config = McpClientConfig::new("https://mcp.example.com")
            .bearer_token("secret")
            .header("x-tenant", "acme")
            .unwrap()
            .headers(headers)
            .http_client(reqwest::Client::new())
            .reinitialize_expired_session(true);
        let debug = format!("{config:?}");
        assert!(debug.contains("[REDACTED]"));
        assert!(!debug.contains("secret"));
        assert!(config.reinitialize_expired_session);
        assert_eq!(config.headers["x-source"], "test");
        assert!(
            McpClientConfig::new("https://mcp.example.com")
                .header("invalid header", "value")
                .is_err()
        );
        assert!(
            McpClientConfig::new("https://mcp.example.com")
                .header("x-value", "invalid\nvalue")
                .is_err()
        );
    }

    #[test]
    fn rejects_non_http_transport_urls_before_network_io() {
        for endpoint in ["file:///tmp/mcp.sock", "relative", "ftp://example.com/mcp"] {
            let error = validate_endpoint(endpoint).unwrap_err();
            assert!(matches!(
                error,
                crate::SdkError::Mcp(McpClientError::InvalidEndpoint(_))
            ));
        }
        assert!(validate_endpoint("http://localhost:8080/mcp").is_ok());
    }

    #[tokio::test]
    async fn connection_rejects_invalid_configuration_before_io() {
        assert!(
            McpClientConfig::new("file:///tmp/mcp.sock")
                .connect()
                .await
                .is_err()
        );
        assert!(
            McpClientConfig::new("https://mcp.example.com")
                .bearer_token(" ")
                .connect()
                .await
                .is_err()
        );
    }
}
