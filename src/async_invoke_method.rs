mod async_invoke;

use crate::async_invoke_method::async_invoke::AsyncInvokeModel;

pub struct ReceiveAsyncInvokeOnlyText {
    response_async_message: Option<String>,
    default_url: String,
    async_invoke_check_url: String,
}

impl ReceiveAsyncInvokeOnlyText {
    pub async fn new(token: &str, message: &str, glm_version:&str, user_config: String) -> Self {
        let default_url = "https://open.bigmodel.cn/api/paas/v4/async/chat/completions".to_string();
        let async_invoke_check_url = "https://open.bigmodel.cn/api/paas/v4/async-result/".to_string();

        let mut instance = Self {
            response_async_message: None,
            default_url,
            async_invoke_check_url,
        };

        instance.send_request_and_wait(token, message, glm_version, user_config).await;
        instance
    }

    pub async fn send_request_and_wait(&mut self, token: &str, message: &str, glm_version:&str, user_config: String) {
        let default_url = self.default_url.clone();
        let async_invoke_check_url = self.async_invoke_check_url.clone();

        let result = AsyncInvokeModel::async_request(token.parse().unwrap(), message.parse().unwrap(), glm_version, user_config, default_url, async_invoke_check_url).await;

        match result {
            Ok(response) => {
                self.response_async_message = Some(response);
            }
            Err(err) => {
                eprintln!("Error: {}", err);
            }
        }
    }

    pub fn get_response(&self) -> Option<&str> {
        self.response_async_message.as_deref()
    }
}
