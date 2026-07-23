use reqwest::Method;
use reqwest::multipart::{Form, Part};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::{Map, Value};

use crate::client::encode_component;
use crate::{RagError, Result, ValidationError, ZhipuClient};

const KNOWLEDGE_PATH: &str = "llm-application/open/knowledge";
const DOCUMENT_PATH: &str = "llm-application/open/document";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KnowledgeEmbeddingModel {
    Embedding2,
    Embedding3,
    Embedding3Pro,
}

impl KnowledgeEmbeddingModel {
    pub const fn id(self) -> u8 {
        match self {
            Self::Embedding2 => 3,
            Self::Embedding3 => 11,
            Self::Embedding3Pro => 12,
        }
    }

    pub const fn code(self) -> &'static str {
        match self {
            Self::Embedding2 => "Embedding-2",
            Self::Embedding3 => "Embedding-3",
            Self::Embedding3Pro => "Embedding-3-pro",
        }
    }
}

impl Serialize for KnowledgeEmbeddingModel {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u8(self.id())
    }
}

impl<'de> Deserialize<'de> for KnowledgeEmbeddingModel {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        match u8::deserialize(deserializer)? {
            3 => Ok(Self::Embedding2),
            11 => Ok(Self::Embedding3),
            12 => Ok(Self::Embedding3Pro),
            value => Err(serde::de::Error::custom(format!(
                "unsupported knowledge embedding model id {value}"
            ))),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum KnowledgeBackground {
    Blue,
    Red,
    Orange,
    Purple,
    Sky,
    Green,
    Yellow,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum KnowledgeIcon {
    Question,
    Book,
    Seal,
    Wrench,
    Tag,
    Horn,
    House,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Contextualization {
    Disabled,
    Enabled,
}

impl Serialize for Contextualization {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u8(matches!(self, Self::Enabled) as u8)
    }
}

impl<'de> Deserialize<'de> for Contextualization {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        match u8::deserialize(deserializer)? {
            0 => Ok(Self::Disabled),
            1 => Ok(Self::Enabled),
            value => Err(serde::de::Error::custom(format!(
                "invalid contextualization value {value}"
            ))),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct KnowledgeCreateRequest {
    pub embedding_id: KnowledgeEmbeddingModel,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contextual: Option<Contextualization>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub background: Option<KnowledgeBackground>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<KnowledgeIcon>,
}

impl KnowledgeCreateRequest {
    pub fn new(name: impl Into<String>, embedding: KnowledgeEmbeddingModel) -> Self {
        Self {
            embedding_id: embedding,
            name: name.into(),
            contextual: None,
            description: None,
            background: None,
            icon: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct KnowledgeUpdateRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embedding_id: Option<KnowledgeEmbeddingModel>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contextual: Option<Contextualization>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub background: Option<KnowledgeBackground>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<KnowledgeIcon>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub callback_url: Option<String>,
    #[serde(default, skip_serializing_if = "Map::is_empty")]
    pub callback_header: Map<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct RagResponse<T> {
    #[serde(default)]
    pub data: Option<T>,
    #[serde(default)]
    pub code: i64,
    #[serde(default)]
    pub message: String,
    #[serde(default)]
    pub timestamp: i64,
    #[serde(flatten, default)]
    pub extra: Map<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct KnowledgeCreated {
    #[serde(default)]
    pub id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct KnowledgeBase {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub embedding_id: u32,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub contextual: u8,
    #[serde(default)]
    pub background: String,
    #[serde(default)]
    pub icon: String,
    #[serde(default)]
    pub document_size: u64,
    #[serde(default)]
    pub length: u64,
    #[serde(default)]
    pub word_num: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct KnowledgeList {
    #[serde(default, rename = "list")]
    pub items: Vec<KnowledgeBase>,
    #[serde(default)]
    pub total: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct CapacityValue {
    #[serde(default)]
    pub word_num: u64,
    #[serde(default)]
    pub length: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct KnowledgeCapacity {
    #[serde(default)]
    pub used: CapacityValue,
    #[serde(default)]
    pub total: CapacityValue,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum RecallMethod {
    Embedding,
    Keyword,
    #[default]
    Mixed,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum RagRerankModel {
    #[serde(rename = "rerank")]
    Rerank,
    #[serde(rename = "rerank-pro")]
    RerankPro,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct KnowledgeRetrieveRequest {
    pub query: String,
    pub knowledge_ids: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub document_ids: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_k: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_n: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recall_method: Option<RecallMethod>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recall_ratio: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rerank_status: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rerank_model: Option<RagRerankModel>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fractional_threshold: Option<f64>,
}

impl KnowledgeRetrieveRequest {
    pub fn new(
        query: impl Into<String>,
        knowledge_ids: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        Self {
            query: query.into(),
            knowledge_ids: knowledge_ids.into_iter().map(Into::into).collect(),
            request_id: None,
            document_ids: Vec::new(),
            top_k: None,
            top_n: None,
            recall_method: None,
            recall_ratio: None,
            rerank_status: None,
            rerank_model: None,
            fractional_threshold: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct RetrievalMetadata {
    #[serde(default, rename = "_id")]
    pub id: String,
    #[serde(default)]
    pub knowledge_id: String,
    #[serde(default)]
    pub doc_id: String,
    #[serde(default)]
    pub doc_name: String,
    #[serde(default)]
    pub doc_url: String,
    #[serde(default)]
    pub contextual_text: String,
    #[serde(flatten, default)]
    pub extra: Map<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct RetrievalMatch {
    #[serde(default)]
    pub text: String,
    #[serde(default)]
    pub score: f64,
    #[serde(default)]
    pub metadata: RetrievalMetadata,
    #[serde(flatten, default)]
    pub extra: Map<String, Value>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DocumentChunking {
    #[default]
    Heading = 1,
    QuestionAnswer = 2,
    Row = 3,
    Custom = 5,
    Page = 6,
    Single = 7,
}

impl DocumentChunking {
    pub const fn code(self) -> u8 {
        self as u8
    }
}

impl Serialize for DocumentChunking {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u8(self.code())
    }
}

impl<'de> Deserialize<'de> for DocumentChunking {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        match u8::deserialize(deserializer)? {
            1 => Ok(Self::Heading),
            2 => Ok(Self::QuestionAnswer),
            3 => Ok(Self::Row),
            5 => Ok(Self::Custom),
            6 => Ok(Self::Page),
            7 => Ok(Self::Single),
            value => Err(serde::de::Error::custom(format!(
                "unsupported document chunking mode {value}"
            ))),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct DocumentFailure {
    #[serde(default)]
    pub embedding_code: i64,
    #[serde(default)]
    pub embedding_msg: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct KnowledgeDocument {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub knowledge_type: u8,
    #[serde(default)]
    pub custom_separator: Vec<String>,
    #[serde(default)]
    pub sentence_size: u32,
    #[serde(default)]
    pub length: u64,
    #[serde(default)]
    pub word_num: u64,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub embedding_stat: i64,
    #[serde(default, rename = "failInfo")]
    pub failure: Option<DocumentFailure>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct DocumentList {
    #[serde(default, rename = "list")]
    pub items: Vec<KnowledgeDocument>,
    #[serde(default)]
    pub total: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentListQuery {
    pub knowledge_id: String,
    pub page: u32,
    pub size: u32,
    pub word: Option<String>,
}

impl DocumentListQuery {
    pub fn new(knowledge_id: impl Into<String>) -> Self {
        Self {
            knowledge_id: knowledge_id.into(),
            page: 1,
            size: 10,
            word: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RagDocumentUpload {
    pub file_name: String,
    pub bytes: Vec<u8>,
    pub mime_type: Option<String>,
    pub chunking: Option<DocumentChunking>,
    pub custom_separator: Vec<String>,
    pub sentence_size: Option<u32>,
    pub parse_image: Option<bool>,
    pub callback_url: Option<String>,
    pub callback_header: Map<String, Value>,
    pub word_num_limit: Option<u64>,
    pub request_id: Option<String>,
}

impl RagDocumentUpload {
    pub fn from_bytes(file_name: impl Into<String>, bytes: impl Into<Vec<u8>>) -> Self {
        Self {
            file_name: file_name.into(),
            bytes: bytes.into(),
            mime_type: None,
            chunking: None,
            custom_separator: Vec::new(),
            sentence_size: None,
            parse_image: None,
            callback_url: None,
            callback_header: Map::new(),
            word_num_limit: None,
            request_id: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct DocumentUploadSuccess {
    #[serde(default, rename = "documentId")]
    pub document_id: String,
    #[serde(default, rename = "fileName")]
    pub file_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct DocumentUploadFailure {
    #[serde(default, rename = "fileName")]
    pub file_name: String,
    #[serde(default, rename = "failReason")]
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct DocumentUploadResult {
    #[serde(default, rename = "successInfos")]
    pub succeeded: Vec<DocumentUploadSuccess>,
    #[serde(default, rename = "failedInfos")]
    pub failed: Vec<DocumentUploadFailure>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct UrlDocument {
    pub url: String,
    pub knowledge_type: DocumentChunking,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub custom_separator: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sentence_size: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub callback_url: Option<String>,
    #[serde(default, skip_serializing_if = "Map::is_empty")]
    pub callback_header: Map<String, Value>,
}

impl UrlDocument {
    pub fn new(url: impl Into<String>, chunking: DocumentChunking) -> Self {
        Self {
            url: url.into(),
            knowledge_type: chunking,
            custom_separator: Vec::new(),
            sentence_size: None,
            callback_url: None,
            callback_header: Map::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct UrlDocumentUploadRequest {
    pub upload_detail: Vec<UrlDocument>,
    pub knowledge_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct UrlUploadSuccess {
    #[serde(default, rename = "documentId")]
    pub document_id: String,
    #[serde(default)]
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct UrlUploadFailure {
    #[serde(default)]
    pub url: String,
    #[serde(default, rename = "failReason")]
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct UrlDocumentUploadResult {
    #[serde(default, rename = "successInfos")]
    pub succeeded: Vec<UrlUploadSuccess>,
    #[serde(default, rename = "failedInfos")]
    pub failed: Vec<UrlUploadFailure>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct DocumentImage {
    #[serde(default)]
    pub text: String,
    #[serde(default)]
    pub cos_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct DocumentImages {
    #[serde(default)]
    pub images: Vec<DocumentImage>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct ReEmbeddingRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub callback_url: Option<String>,
    #[serde(default, skip_serializing_if = "Map::is_empty")]
    pub callback_header: Map<String, Value>,
}

pub type KnowledgeCreateResponse = RagResponse<KnowledgeCreated>;
pub type KnowledgeListResponse = RagResponse<KnowledgeList>;
pub type KnowledgeDetailResponse = RagResponse<KnowledgeBase>;
pub type KnowledgeCapacityResponse = RagResponse<KnowledgeCapacity>;
pub type KnowledgeRetrieveResponse = RagResponse<Vec<RetrievalMatch>>;
pub type KnowledgeDocumentListResponse = RagResponse<DocumentList>;
pub type KnowledgeDocumentResponse = RagResponse<KnowledgeDocument>;
pub type DocumentUploadResponse = RagResponse<DocumentUploadResult>;
pub type UrlDocumentUploadResponse = RagResponse<UrlDocumentUploadResult>;
pub type DocumentImagesResponse = RagResponse<DocumentImages>;
pub type RagOperationResponse = RagResponse<()>;

impl ZhipuClient {
    pub async fn create_knowledge_base(
        &self,
        request: &KnowledgeCreateRequest,
    ) -> Result<KnowledgeCreateResponse> {
        if request.name.trim().is_empty() {
            return Err(RagError::EmptyKnowledgeName.into());
        }
        self.agent_transport
            .post_json(KNOWLEDGE_PATH, request)
            .await
    }

    pub async fn knowledge_bases(&self, page: u32, size: u32) -> Result<KnowledgeListResponse> {
        validate_pagination(page, size)?;
        self.agent_transport
            .get_json(&format!("{KNOWLEDGE_PATH}?page={page}&size={size}"))
            .await
    }

    pub async fn knowledge_base(&self, id: &str) -> Result<KnowledgeDetailResponse> {
        require_rag_id(id, "knowledge id")?;
        self.agent_transport
            .get_json(&format!("{KNOWLEDGE_PATH}/{}", encode_component(id)))
            .await
    }

    pub async fn update_knowledge_base(
        &self,
        id: &str,
        request: &KnowledgeUpdateRequest,
    ) -> Result<RagOperationResponse> {
        require_rag_id(id, "knowledge id")?;
        if request
            .name
            .as_ref()
            .is_some_and(|name| name.trim().is_empty())
        {
            return Err(RagError::EmptyKnowledgeName.into());
        }
        self.agent_transport
            .request_json(
                Method::PUT,
                &format!("{KNOWLEDGE_PATH}/{}", encode_component(id)),
                Some(request),
            )
            .await
    }

    pub async fn delete_knowledge_base(&self, id: &str) -> Result<RagOperationResponse> {
        require_rag_id(id, "knowledge id")?;
        self.agent_transport
            .delete_json(&format!("{KNOWLEDGE_PATH}/{}", encode_component(id)))
            .await
    }

    pub async fn knowledge_capacity(&self) -> Result<KnowledgeCapacityResponse> {
        self.agent_transport
            .get_json(&format!("{KNOWLEDGE_PATH}/capacity"))
            .await
    }

    pub async fn retrieve_knowledge(
        &self,
        request: &KnowledgeRetrieveRequest,
    ) -> Result<KnowledgeRetrieveResponse> {
        validate_retrieval(request)?;
        self.agent_transport
            .post_json(&format!("{KNOWLEDGE_PATH}/retrieve"), request)
            .await
    }

    pub async fn knowledge_documents(
        &self,
        query: &DocumentListQuery,
    ) -> Result<KnowledgeDocumentListResponse> {
        require_rag_id(&query.knowledge_id, "knowledge id")?;
        validate_pagination(query.page, query.size)?;
        let mut path = format!(
            "{DOCUMENT_PATH}?knowledge_id={}&page={}&size={}",
            encode_component(&query.knowledge_id),
            query.page,
            query.size
        );
        if let Some(word) = query.word.as_deref() {
            path.push_str("&word=");
            path.push_str(&encode_component(word));
        }
        self.agent_transport.get_json(&path).await
    }

    pub async fn upload_knowledge_document(
        &self,
        knowledge_id: &str,
        request: RagDocumentUpload,
    ) -> Result<DocumentUploadResponse> {
        require_rag_id(knowledge_id, "knowledge id")?;
        if request.file_name.trim().is_empty() {
            return Err(RagError::InvalidField {
                field: "file_name",
                reason: "cannot be empty".into(),
            }
            .into());
        }
        if request.bytes.is_empty() {
            return Err(RagError::InvalidField {
                field: "bytes",
                reason: "cannot be empty".into(),
            }
            .into());
        }

        let mut part = Part::bytes(request.bytes).file_name(request.file_name);
        if let Some(mime_type) = request.mime_type {
            part = part
                .mime_str(&mime_type)
                .map_err(|error| RagError::InvalidField {
                    field: "mime_type",
                    reason: error.to_string(),
                })?;
        }
        let mut form = Form::new().part("files", part);
        if let Some(chunking) = request.chunking {
            form = form.text("knowledge_type", chunking.code().to_string());
        }
        if !request.custom_separator.is_empty() {
            form = form.text(
                "custom_separator",
                serde_json::to_string(&request.custom_separator)
                    .map_err(|error| ValidationError::Serialization(error.to_string()))?,
            );
        }
        if let Some(value) = request.sentence_size {
            form = form.text("sentence_size", value.to_string());
        }
        if let Some(value) = request.parse_image {
            form = form.text("parse_image", value.to_string());
        }
        if let Some(value) = request.callback_url {
            form = form.text("callback_url", value);
        }
        if !request.callback_header.is_empty() {
            form = form.text(
                "callback_header",
                serde_json::to_string(&request.callback_header)
                    .map_err(|error| ValidationError::Serialization(error.to_string()))?,
            );
        }
        if let Some(value) = request.word_num_limit {
            form = form.text("word_num_limit", value.to_string());
        }
        if let Some(value) = request.request_id {
            form = form.text("req_id", value);
        }

        self.agent_transport
            .post_multipart(
                &format!(
                    "{DOCUMENT_PATH}/upload_document/{}",
                    encode_component(knowledge_id)
                ),
                form,
            )
            .await
    }

    pub async fn upload_knowledge_urls(
        &self,
        request: &UrlDocumentUploadRequest,
    ) -> Result<UrlDocumentUploadResponse> {
        require_rag_id(&request.knowledge_id, "knowledge id")?;
        if request.upload_detail.is_empty() {
            return Err(RagError::InvalidField {
                field: "upload_detail",
                reason: "must contain at least one URL".into(),
            }
            .into());
        }
        if request
            .upload_detail
            .iter()
            .any(|document| document.url.trim().is_empty())
        {
            return Err(RagError::InvalidField {
                field: "upload_detail.url",
                reason: "cannot be empty".into(),
            }
            .into());
        }
        self.agent_transport
            .post_json(&format!("{DOCUMENT_PATH}/upload_url"), request)
            .await
    }

    pub async fn knowledge_document(&self, id: &str) -> Result<KnowledgeDocumentResponse> {
        require_rag_id(id, "document id")?;
        self.agent_transport
            .get_json(&format!("{DOCUMENT_PATH}/{}", encode_component(id)))
            .await
    }

    pub async fn delete_knowledge_document(&self, id: &str) -> Result<RagOperationResponse> {
        require_rag_id(id, "document id")?;
        self.agent_transport
            .delete_json(&format!("{DOCUMENT_PATH}/{}", encode_component(id)))
            .await
    }

    pub async fn knowledge_document_images(&self, id: &str) -> Result<DocumentImagesResponse> {
        require_rag_id(id, "document id")?;
        self.agent_transport
            .request_json::<Value, _>(
                Method::POST,
                &format!("{DOCUMENT_PATH}/slice/image_list/{}", encode_component(id)),
                None,
            )
            .await
    }

    pub async fn reembed_knowledge_document(
        &self,
        id: &str,
        request: &ReEmbeddingRequest,
    ) -> Result<RagOperationResponse> {
        require_rag_id(id, "document id")?;
        self.agent_transport
            .post_json(
                &format!("{DOCUMENT_PATH}/embedding/{}", encode_component(id)),
                request,
            )
            .await
    }
}

fn validate_pagination(page: u32, size: u32) -> Result<()> {
    if page == 0 || !(1..=100).contains(&size) {
        return Err(RagError::InvalidPagination.into());
    }
    Ok(())
}

fn validate_retrieval(request: &KnowledgeRetrieveRequest) -> Result<()> {
    if request.query.trim().is_empty() {
        return Err(RagError::InvalidField {
            field: "query",
            reason: "cannot be empty".into(),
        }
        .into());
    }
    if request.knowledge_ids.is_empty()
        || request.knowledge_ids.iter().any(|id| id.trim().is_empty())
    {
        return Err(RagError::EmptyKnowledgeIds.into());
    }
    if request
        .top_k
        .is_some_and(|value| !(1..=20).contains(&value))
    {
        return Err(RagError::InvalidField {
            field: "top_k",
            reason: "expected 1..=20".into(),
        }
        .into());
    }
    if request
        .top_n
        .is_some_and(|value| !(1..=100).contains(&value))
    {
        return Err(RagError::InvalidField {
            field: "top_n",
            reason: "expected 1..=100".into(),
        }
        .into());
    }
    if request.recall_ratio.is_some_and(|value| value > 100) {
        return Err(RagError::InvalidField {
            field: "recall_ratio",
            reason: "expected 0..=100".into(),
        }
        .into());
    }
    if request
        .fractional_threshold
        .is_some_and(|value| !(0.0..=1.0).contains(&value))
    {
        return Err(RagError::InvalidField {
            field: "fractional_threshold",
            reason: "expected 0.0..=1.0".into(),
        }
        .into());
    }
    Ok(())
}

fn require_rag_id(value: &str, field: &'static str) -> Result<()> {
    if value.trim().is_empty() {
        return Err(RagError::InvalidField {
            field,
            reason: "cannot be empty".into(),
        }
        .into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serializes_official_embedding_and_chunking_codes() {
        let request =
            KnowledgeCreateRequest::new("engineering", KnowledgeEmbeddingModel::Embedding3Pro);
        let value = serde_json::to_value(request).unwrap();
        assert_eq!(value["embedding_id"], 12);

        let url = UrlDocument::new("https://example.com/runbook", DocumentChunking::Heading);
        assert_eq!(serde_json::to_value(url).unwrap()["knowledge_type"], 1);
    }

    #[test]
    fn rejects_invalid_retrieval_ranges() {
        let mut request = KnowledgeRetrieveRequest::new("query", ["knowledge-1"]);
        request.top_k = Some(21);
        assert!(matches!(
            validate_retrieval(&request),
            Err(crate::SdkError::Rag(RagError::InvalidField {
                field: "top_k",
                ..
            }))
        ));
    }

    #[test]
    fn document_upload_is_memory_owned() {
        let request = RagDocumentUpload::from_bytes("runbook.md", b"content".to_vec());
        assert_eq!(request.bytes, b"content");
        assert_eq!(request.file_name, "runbook.md");
    }
}
