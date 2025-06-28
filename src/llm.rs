use async_openai::{
    Client,
    config::OpenAIConfig,
    types::{
        ChatCompletionRequestMessage, ChatCompletionRequestUserMessageArgs,
        ChatCompletionRequestUserMessageContent, ChatCompletionStreamOptions,
        CreateChatCompletionRequestArgs,
    },
};
use futures_util::StreamExt;
use log::{debug, error, info};
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
        let request = CreateChatCompletionRequestArgs::default()
            .model(&self.model_name)
            .temperature(0.7)
            .stream(true)
            .messages(vec![user_message(&request.text)?])
            .build()?;

        let mut gradual_response = String::new();

        RT.block_on(async {
            let gradual_response = &mut gradual_response;
            let mut stream = self.client.chat().create_stream(request).await.unwrap();

            while let Some(event) = stream.next().await {
                match event {
                    Ok(chunk) => {
                        debug!("Received chunk: {chunk:?}");

                        if let Some(delta) =
                            chunk.choices.first().and_then(|c| c.delta.content.as_ref())
                        {
                            gradual_response.push_str(delta);
                        }
                    }
                    Err(e) => {
                        error!("Error in stream: {e}");
                        break;
                    }
                }
            }
        });

        info!("Response: {gradual_response}");

        Ok(LlmResponse {
            text: gradual_response,
        })
    }
}

fn user_message(
    content: impl Into<String>,
) -> Result<ChatCompletionRequestMessage, Box<dyn std::error::Error>> {
    let message = ChatCompletionRequestUserMessageArgs::default()
        .content(ChatCompletionRequestUserMessageContent::Text(
            content.into(),
        ))
        .build()?;

    Ok(message.into())
}
