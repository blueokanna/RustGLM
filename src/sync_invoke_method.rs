mod sync_invoke;

#[derive(Debug)]
pub struct ReceiveInvokeModelOnlyText {
    response_sync_message: Option<String>,
    default_url: String,
}

impl ReceiveInvokeModelOnlyText {
    pub async fn new(token: &str, message: &str, user_config: &str) -> Self {
        let default_url = "https://open.bigmodel.cn/api/paas/v4/chat/completions".trim().to_string();

        let mut instance = Self {
            response_sync_message: None,
            default_url,
        };

        instance.send_request_and_wait(token, message, user_config).await;
        instance
    }
    pub async fn send_request_and_wait(&mut self, token: &str, message: &str, user_config: &str) {
        let default_url = self.default_url.clone();

        let result = sync_invoke::SyncInvokeModel::sync_request(token.parse().unwrap(), message.parse().unwrap(), user_config, default_url);

        match result.await {
            Ok(response) => {
                self.response_sync_message = Some(response);
            }
            Err(err) => {
                eprintln!("Error: {}", err);
            }
        }
    }


    pub fn get_response_message(&self) -> Option<&str> {
        self.response_sync_message.as_deref()
    }
}
