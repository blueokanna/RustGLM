use rustglm::{ChatCompletionRequest, ChatMessage, ZhipuClient};

#[tokio::test]
#[ignore = "requires ZHIPU_API_KEY and performs a billable network request"]
async fn live_chat_completion() {
    let api_key = std::env::var("ZHIPU_API_KEY").expect("ZHIPU_API_KEY is required");
    let client = ZhipuClient::new(api_key).expect("client configuration must be valid");
    let request = ChatCompletionRequest::new("glm-4-flash")
        .message(ChatMessage::user("只回复 RustGLM-live-test"));
    let response = client
        .chat_completion(&request)
        .await
        .expect("live request must succeed");
    assert!(!response.choices.is_empty());
    assert!(response.text().is_some_and(|text| !text.trim().is_empty()));
}
