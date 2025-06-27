use async_openai::{
    Client,
    config::OpenAIConfig,
    types::{
        ChatCompletionRequestMessage, ChatCompletionRequestUserMessage, CreateChatCompletionRequest,
    },
};
use log::info;
use tokio::runtime::Runtime;

pub struct LlmClient {
    client: Client<OpenAIConfig>,
    model_name: String,
}

pub struct LlmRequest {
    pub text: String,
}

pub struct LlmResponse {
    pub text: String,
}

static RT: std::sync::LazyLock<Runtime> = std::sync::LazyLock::new(|| {
    tokio::runtime::Builder::new_current_thread() // one thread, no scheduler pool
        .enable_all() // timers, TCP, etc.
        .build()
        .expect("failed to build Tokio runtime")
});

impl LlmClient {
    pub fn new(
        model_name: impl Into<String>,
        api_key: impl Into<String>,
        base_uri: impl Into<String>,
    ) -> Self {
        let config = OpenAIConfig::new()
            .with_api_key(api_key)
            .with_api_base(base_uri);

        let client = Client::with_config(config);
        let model_name = model_name.into();

        Self { client, model_name }
    }

    pub fn get_chat_completion(
        &self,
        request: &LlmRequest,
    ) -> Result<LlmResponse, Box<dyn std::error::Error>> {
        let request = CreateChatCompletionRequest {
            model: self.model_name.clone(),
            temperature: Some(0.7),
            messages: vec![ChatCompletionRequestMessage::User(
                ChatCompletionRequestUserMessage {
                    content: async_openai::types::ChatCompletionRequestUserMessageContent::Text(
                        request.text.to_owned(),
                    ),
                    name: None,
                },
            )],
            ..Default::default()
        };

        let response = RT.block_on(async { self.client.chat().create(request).await })?;

        info!("Response: {response:?}");

        let first_choice = response.choices.into_iter().next().unwrap();

        Ok(LlmResponse {
            text: first_choice.message.content.unwrap(),
        })
    }
}
