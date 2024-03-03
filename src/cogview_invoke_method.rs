mod cogview_invoke;

#[derive(Debug)]
pub struct ReceiveCogviewInvokeModel {
    response_cogview_message: Option<String>,
    default_url: String,
}

impl ReceiveCogviewInvokeModel {
    pub async fn new(token: &str, message: &str, user_config: &str) -> Self {
        let default_url = "https://open.bigmodel.cn/api/paas/v4/images/generations".trim().to_string();

        let mut instance = Self {
            response_cogview_message: None,
            default_url,
        };

        instance.send_request_and_wait(token, message, user_config).await;
        instance
    }
    pub async fn send_request_and_wait(&mut self, token: &str, user_config: &str, message: &str) {
        let default_url = self.default_url.clone();

        let cogview_result = cogview_invoke::CogViewInvokeModel::cogview_request(token.parse().unwrap(), message.parse().unwrap(), user_config, default_url);

        match cogview_result.await {
            Ok(response) => {
                self.response_cogview_message = Some(response);
            }
            Err(err) => {
                eprintln!("Error: {}", err);
            }
        }
    }


    pub fn get_cogview_response_message(&self) -> Option<&str> {
        self.response_cogview_message.as_deref()
    }
}