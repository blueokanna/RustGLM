mod glm4v_invoke;

#[derive(Debug)]
pub struct Receive4VInvokeModelwithText {
    response_glm4v_message: Option<String>,
    default_4vurl: String,
}

impl Receive4VInvokeModelwithText {
    pub async fn new(token: &str, message: &str, user_config: &str) -> Self {
        let default_4vurl = "https://open.bigmodel.cn/api/paas/v4/chat/completions".trim().to_string();

        let mut instance = Self {
            response_glm4v_message: None,
            default_4vurl,
        };

        instance.send_request_and_wait(token, message, user_config).await;
        instance
    }
    pub async fn send_request_and_wait(&mut self, token: &str, message: &str, user_config: &str) {
        let default_url = self.default_4vurl.clone();

        let result = glm4v_invoke::GLM4vInvokeModel::glm4v_request(token.parse().unwrap(), message.parse().unwrap(), user_config, default_url);

        match result.await {
            Ok(response) => {
                self.response_glm4v_message = Some(response);
            }
            Err(err) => {
                eprintln!("{}", err);
            }
        }
    }


    pub fn get_response_glm4v_message(&self) -> Option<&str> {
        self.response_glm4v_message.as_deref()
    }
}
