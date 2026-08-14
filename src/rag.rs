use reqwest::Method;
use reqwest::multipart::{Form, Part};
use nextjson::{NsonDeserialize as Deserialize, NsonSerialize as Serialize};
use nextjson::{Map, Value};
use nextjson::FormatError;

use crate::wire_enum;

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
    fn nextencode<E: nextjson::FormatEncoder>(
        &self,
        encoder: &mut E,
    ) -> std::result::Result<(), E::Error> {
        encoder.write_u8(self.id())
    }
}

impl nextjson::NsonSchema for KnowledgeEmbeddingModel {
    const SCHEMA: nextjson::TypeSchema = nextjson::TypeSchema::U8;
}

impl<'de> Deserialize<'de> for KnowledgeEmbeddingModel {
    fn nextdecode_into<D: nextjson::FormatDecoder<'de>>(
        decoder: &mut D,
        out: &mut nextjson::DecodeSlot<Self>,
    ) -> std::result::Result<(), D::Error> {
        match decoder.u8()? {
            3 => out.write(Self::Embedding2),
            11 => out.write(Self::Embedding3),
            12 => out.write(Self::Embedding3Pro),
            value => {
                return Err(D::Error::custom(format!(
                    "unsupported knowledge embedding model id {value}"
                )))
            }
        }
        Ok(())
    }
}

wire_enum! {
    /// Knowledge base background color.
    pub enum KnowledgeBackground {
        Blue => "blue",
        Red => "red",
        Orange => "orange",
        Purple => "purple",
        Sky => "sky",
        Green => "green",
        Yellow => "yellow",
    }
}

wire_enum! {
    /// Knowledge base icon.
    pub enum KnowledgeIcon {
        Question => "question",
        Book => "book",
        Seal => "seal",
        Wrench => "wrench",
        Tag => "tag",
        Horn => "horn",
        House => "house",
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Contextualization {
    Disabled,
    Enabled,
}

impl Serialize for Contextualization {
    fn nextencode<E: nextjson::FormatEncoder>(
        &self,
        encoder: &mut E,
    ) -> std::result::Result<(), E::Error> {
        encoder.write_u8(matches!(self, Self::Enabled) as u8)
    }
}

impl nextjson::NsonSchema for Contextualization {
    const SCHEMA: nextjson::TypeSchema = nextjson::TypeSchema::U8;
}

impl<'de> Deserialize<'de> for Contextualization {
    fn nextdecode_into<D: nextjson::FormatDecoder<'de>>(
        decoder: &mut D,
        out: &mut nextjson::DecodeSlot<Self>,
    ) -> std::result::Result<(), D::Error> {
        match decoder.u8()? {
            0 => out.write(Self::Disabled),
            1 => out.write(Self::Enabled),
            value => {
                return Err(D::Error::custom(format!(
                    "invalid contextualization value {value}"
                )))
            }
        }
        Ok(())
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
    pub callback_header: Map,
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
    pub extra: Map,
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

wire_enum! {
    /// Knowledge recall method.
    pub enum RecallMethod {
        Embedding => "embedding",
        Keyword => "keyword",
        Mixed => "mixed",
    }
}

#[allow(clippy::derivable_impls)]
impl Default for RecallMethod {
    fn default() -> Self {
        Self::Mixed
    }
}

wire_enum! {
    /// Rerank model identifier.
    pub enum RagRerankModel {
        Rerank => "rerank",
        RerankPro => "rerank-pro",
    }
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
    pub extra: Map,
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
    pub extra: Map,
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
    fn nextencode<E: nextjson::FormatEncoder>(
        &self,
        encoder: &mut E,
    ) -> std::result::Result<(), E::Error> {
        encoder.write_u8(self.code())
    }
}

impl nextjson::NsonSchema for DocumentChunking {
    const SCHEMA: nextjson::TypeSchema = nextjson::TypeSchema::U8;
}

impl<'de> Deserialize<'de> for DocumentChunking {
    fn nextdecode_into<D: nextjson::FormatDecoder<'de>>(
        decoder: &mut D,
        out: &mut nextjson::DecodeSlot<Self>,
    ) -> std::result::Result<(), D::Error> {
        match decoder.u8()? {
            1 => out.write(Self::Heading),
            2 => out.write(Self::QuestionAnswer),
            3 => out.write(Self::Row),
            5 => out.write(Self::Custom),
            6 => out.write(Self::Page),
            7 => out.write(Self::Single),
            value => {
                return Err(D::Error::custom(format!(
                    "unsupported document chunking mode {value}"
                )))
            }
        }
        Ok(())
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
    pub callback_header: Map,
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
    pub callback_header: Map,
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
    pub callback_header: Map,
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
                nextjson::to_string(&request.custom_separator)
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
                nextjson::to_string(&request.callback_header)
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
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    use super::*;

    async fn mock_server(
        bodies: Vec<&'static str>,
    ) -> (String, tokio::task::JoinHandle<Vec<String>>) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let mut requests = Vec::new();
            for body in bodies {
                let (mut socket, _) = listener.accept().await.unwrap();
                let mut request = Vec::new();
                let mut buffer = [0_u8; 4096];
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
                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                );
                socket.write_all(response.as_bytes()).await.unwrap();
            }
            requests
        });
        (format!("http://{address}"), server)
    }

    #[test]
    fn serializes_official_embedding_and_chunking_codes() {
        let request =
            KnowledgeCreateRequest::new("engineering", KnowledgeEmbeddingModel::Embedding3Pro);
        let value = nextjson::to_value(&request).unwrap();
        assert_eq!(value["embedding_id"].as_u64(), Some(12));

        let url = UrlDocument::new("https://example.com/runbook", DocumentChunking::Heading);
        assert_eq!(
            nextjson::to_value(&url).unwrap()["knowledge_type"].as_u64(),
            Some(1)
        );
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

    #[test]
    fn validates_all_rag_boundaries_and_wire_enums() {
        assert!(matches!(
            validate_pagination(0, 10),
            Err(crate::SdkError::Rag(RagError::InvalidPagination))
        ));
        assert!(validate_pagination(1, 100).is_ok());
        assert!(require_rag_id(" ", "document id").is_err());

        for (mut request, field) in [
            (KnowledgeRetrieveRequest::new(" ", ["knowledge"]), "query"),
            (
                KnowledgeRetrieveRequest::new("query", [""]),
                "knowledge_ids",
            ),
        ] {
            let error = validate_retrieval(&request).unwrap_err();
            match (error, field) {
                (crate::SdkError::Rag(RagError::InvalidField { field: "query", .. }), "query")
                | (crate::SdkError::Rag(RagError::EmptyKnowledgeIds), "knowledge_ids") => {}
                other => panic!("unexpected validation result: {other:?}"),
            }
            request.query = "query".into();
        }

        let mut request = KnowledgeRetrieveRequest::new("query", ["knowledge"]);
        request.top_n = Some(101);
        assert!(validate_retrieval(&request).is_err());
        request.top_n = None;
        request.recall_ratio = Some(101);
        assert!(validate_retrieval(&request).is_err());
        request.recall_ratio = None;
        request.fractional_threshold = Some(f64::NAN);
        assert!(validate_retrieval(&request).is_err());

        assert_eq!(
            nextjson::from_str::<KnowledgeEmbeddingModel>("3").unwrap(),
            KnowledgeEmbeddingModel::Embedding2
        );
        assert_eq!(
            nextjson::from_str::<KnowledgeEmbeddingModel>("11").unwrap(),
            KnowledgeEmbeddingModel::Embedding3
        );
        assert!(nextjson::from_str::<KnowledgeEmbeddingModel>("99").is_err());
        assert_eq!(
            nextjson::from_str::<Contextualization>("0").unwrap(),
            Contextualization::Disabled
        );
        assert_eq!(
            nextjson::from_str::<Contextualization>("1").unwrap(),
            Contextualization::Enabled
        );
        assert!(nextjson::from_str::<Contextualization>("2").is_err());
        for value in [1, 2, 3, 5, 6, 7] {
            let chunking =
                nextjson::from_value::<DocumentChunking>(nextjson::Value::from(value)).unwrap();
            assert_eq!(u64::from(chunking.code()), value);
        }
        assert!(nextjson::from_str::<DocumentChunking>("4").is_err());
    }

    #[tokio::test]
    async fn calls_every_knowledge_and_document_endpoint() {
        let (base_url, server) = mock_server(vec![
            r#"{"data":{"id":"kb"}}"#,
            r#"{"data":{"list":[{"id":"kb"}],"total":1}}"#,
            r#"{"data":{"id":"kb"}}"#,
            r#"{}"#,
            r#"{}"#,
            r#"{"data":{"used":{},"total":{}}}"#,
            r#"{"data":[{"text":"answer","score":0.9,"metadata":{}}]}"#,
            r#"{"data":{"list":[{"id":"doc"}],"total":1}}"#,
            r#"{"data":{"successInfos":[{"documentId":"doc","fileName":"runbook.md"}]}}"#,
            r#"{"data":{"successInfos":[{"documentId":"url-doc","url":"https://example.com"}]}}"#,
            r#"{"data":{"id":"doc"}}"#,
            r#"{}"#,
            r#"{"data":{"images":[{"text":"diagram","cos_url":"https://example.com/image"}]}}"#,
            r#"{}"#,
        ])
        .await;
        let client = crate::ZhipuConfig::new("test-key")
            .agent_base_url(&base_url)
            .build()
            .unwrap();

        let create = KnowledgeCreateRequest::new("runbooks", KnowledgeEmbeddingModel::Embedding3);
        assert_eq!(
            client
                .create_knowledge_base(&create)
                .await
                .unwrap()
                .data
                .unwrap()
                .id,
            "kb"
        );
        assert_eq!(
            client
                .knowledge_bases(1, 20)
                .await
                .unwrap()
                .data
                .unwrap()
                .total,
            1
        );
        assert_eq!(
            client
                .knowledge_base("kb / one")
                .await
                .unwrap()
                .data
                .unwrap()
                .id,
            "kb"
        );
        let update = KnowledgeUpdateRequest {
            name: Some("updated".into()),
            ..KnowledgeUpdateRequest::default()
        };
        client.update_knowledge_base("kb", &update).await.unwrap();
        client.delete_knowledge_base("kb").await.unwrap();
        assert!(client.knowledge_capacity().await.unwrap().data.is_some());

        let mut retrieval = KnowledgeRetrieveRequest::new("question", ["kb"]);
        retrieval.top_k = Some(20);
        retrieval.top_n = Some(100);
        retrieval.recall_ratio = Some(100);
        retrieval.fractional_threshold = Some(0.5);
        assert_eq!(
            client
                .retrieve_knowledge(&retrieval)
                .await
                .unwrap()
                .data
                .unwrap()[0]
                .text,
            "answer"
        );

        let query = DocumentListQuery {
            knowledge_id: "kb".into(),
            page: 1,
            size: 20,
            word: Some("run book".into()),
        };
        assert_eq!(
            client
                .knowledge_documents(&query)
                .await
                .unwrap()
                .data
                .unwrap()
                .total,
            1
        );

        let mut upload = RagDocumentUpload::from_bytes("runbook.md", b"content".to_vec());
        upload.mime_type = Some("text/markdown".into());
        upload.chunking = Some(DocumentChunking::Custom);
        upload.custom_separator = vec!["---".into()];
        upload.sentence_size = Some(500);
        upload.parse_image = Some(true);
        upload.callback_url = Some("https://example.com/callback".into());
        upload
            .callback_header
            .insert("x-job".into(), Value::String("one".into()));
        upload.word_num_limit = Some(10_000);
        upload.request_id = Some("request-one".into());
        assert_eq!(
            client
                .upload_knowledge_document("kb", upload)
                .await
                .unwrap()
                .data
                .unwrap()
                .succeeded[0]
                .document_id,
            "doc"
        );

        let url_request = UrlDocumentUploadRequest {
            knowledge_id: "kb".into(),
            upload_detail: vec![UrlDocument::new(
                "https://example.com",
                DocumentChunking::Heading,
            )],
        };
        assert_eq!(
            client
                .upload_knowledge_urls(&url_request)
                .await
                .unwrap()
                .data
                .unwrap()
                .succeeded[0]
                .document_id,
            "url-doc"
        );
        assert_eq!(
            client
                .knowledge_document("doc")
                .await
                .unwrap()
                .data
                .unwrap()
                .id,
            "doc"
        );
        client.delete_knowledge_document("doc").await.unwrap();
        assert_eq!(
            client
                .knowledge_document_images("doc")
                .await
                .unwrap()
                .data
                .unwrap()
                .images
                .len(),
            1
        );
        client
            .reembed_knowledge_document("doc", &ReEmbeddingRequest::default())
            .await
            .unwrap();

        let requests = server.await.unwrap();
        assert!(requests[0].starts_with("POST /llm-application/open/knowledge "));
        assert!(requests[1].starts_with("GET /llm-application/open/knowledge?page=1&size=20 "));
        assert!(requests[2].contains("/knowledge/kb%20%2F%20one"));
        assert!(requests[3].starts_with("PUT /llm-application/open/knowledge/kb "));
        assert!(requests[4].starts_with("DELETE /llm-application/open/knowledge/kb "));
        assert!(requests[7].contains("word=run%20book"));
        assert!(
            requests[8]
                .to_ascii_lowercase()
                .contains("multipart/form-data")
        );
        assert!(
            requests[12].starts_with("POST /llm-application/open/document/slice/image_list/doc ")
        );
    }

    #[tokio::test]
    async fn endpoint_validation_fails_before_network_io() {
        let client = crate::ZhipuClient::new("test-key").unwrap();
        assert!(
            client
                .create_knowledge_base(&KnowledgeCreateRequest::new(
                    " ",
                    KnowledgeEmbeddingModel::Embedding2,
                ))
                .await
                .is_err()
        );
        let update = KnowledgeUpdateRequest {
            name: Some(" ".into()),
            ..KnowledgeUpdateRequest::default()
        };
        assert!(client.update_knowledge_base("kb", &update).await.is_err());
        assert!(client.knowledge_base("").await.is_err());
        assert!(
            client
                .knowledge_documents(&DocumentListQuery::new(""))
                .await
                .is_err()
        );
        assert!(
            client
                .upload_knowledge_document("kb", RagDocumentUpload::from_bytes("", b"x"))
                .await
                .is_err()
        );
        assert!(
            client
                .upload_knowledge_document("kb", RagDocumentUpload::from_bytes("file", []))
                .await
                .is_err()
        );
        let mut invalid_mime = RagDocumentUpload::from_bytes("file", b"x");
        invalid_mime.mime_type = Some("not a mime\n".into());
        assert!(
            client
                .upload_knowledge_document("kb", invalid_mime)
                .await
                .is_err()
        );
        assert!(
            client
                .upload_knowledge_urls(&UrlDocumentUploadRequest {
                    knowledge_id: "kb".into(),
                    upload_detail: Vec::new(),
                })
                .await
                .is_err()
        );
        assert!(
            client
                .upload_knowledge_urls(&UrlDocumentUploadRequest {
                    knowledge_id: "kb".into(),
                    upload_detail: vec![UrlDocument::new(" ", DocumentChunking::Heading)],
                })
                .await
                .is_err()
        );
        assert!(client.knowledge_document("").await.is_err());
        assert!(client.delete_knowledge_document("").await.is_err());
        assert!(client.knowledge_document_images("").await.is_err());
        assert!(
            client
                .reembed_knowledge_document("", &ReEmbeddingRequest::default())
                .await
                .is_err()
        );
    }
}
