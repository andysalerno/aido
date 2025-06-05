use async_openai::{Client, config::OpenAIConfig, types::CreateChatCompletionRequest};

pub struct LlmClient {
    client: Client<OpenAIConfig>,
}

impl LlmClient {
    pub fn new(api_key: impl Into<String>, base_uri: impl Into<String>) -> Self {
        let config = OpenAIConfig::new()
            .with_api_key(api_key)
            .with_api_base(base_uri);

        let client = Client::with_config(config);

        Self { client }
    }

    pub async fn get_chat_completion(&self) {
        let request = CreateChatCompletionRequest {
            model: "testing".to_owned(),
            temperature: Some(0.7),
            ..Default::default()
        };

        let response = self.client.chat().create(request).await;
    }
}
