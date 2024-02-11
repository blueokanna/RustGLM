mod sse_invoke;

use reqwest;
use std::error::Error;

#[derive(Debug)]
pub struct ReceiveSSEInvokeModelOnlyText {
    response_sse_message:  Option<String>,
    default_url: String,
}

impl ReceiveSSEInvokeModelOnlyText {
    pub async fn new(token: &str, message: &str) -> Self {
        let default_url = "https://open.bigmodel.cn/api/paas/v4/chat/completions".trim().to_string();

        let mut instance = Self {
            response_sse_message: None,
            default_url,
        };

        instance.send_request_and_wait(token, message).await;
        instance
    }
    pub async fn send_request_and_wait(&mut self, token: &str, message: &str) {
        let default_url = self.default_url.clone();

        let result = sse_invoke::SSEInvokeModel::sse_request(token.parse().unwrap(), message.parse().unwrap(), default_url);

        match result.await {
            Ok(response) => {
                self.response_sse_message = Some(response);
            }
            Err(err) => {
                eprintln!("Error: {}", err);
            }
        }
    }


    pub fn get_response_message(&self) -> Option<&str> {
        self.response_sse_message.as_deref()
    }

}
