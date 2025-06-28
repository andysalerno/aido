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
use log::{debug, error};
use tokio::runtime::Runtime;

pub struct LlmClient {
    client: Client<OpenAIConfig>,
    model_name: String,
}

pub struct LlmRequest {
    pub text: String,
}

#[derive(Debug, Clone, Default)]
pub struct LlmResponse {
    text: String,
    usage: Usage,
}

impl LlmResponse {
    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn usage(&self) -> &Usage {
        &self.usage
    }
}

#[derive(Debug, Clone, Default)]
pub struct Usage {
    prompt_tokens: u32,
    completion_tokens: u32,
    total_tokens: u32,
}

impl Usage {
    pub fn new(prompt_tokens: u32, completion_tokens: u32, total_tokens: u32) -> Self {
        Self {
            prompt_tokens,
            completion_tokens,
            total_tokens,
        }
    }

    pub fn prompt_tokens(&self) -> u32 {
        self.prompt_tokens
    }

    pub fn completion_tokens(&self) -> u32 {
        self.completion_tokens
    }

    pub fn total_tokens(&self) -> u32 {
        self.total_tokens
    }
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

    pub fn get_chat_completion_streaming(
        &self,
        request: &LlmRequest,
        mut action_per_chunk: impl FnMut(&str),
    ) -> Result<LlmResponse, Box<dyn std::error::Error>> {
        let request = CreateChatCompletionRequestArgs::default()
            .model(&self.model_name)
            .temperature(0.7)
            .stream(true)
            .stream_options(ChatCompletionStreamOptions {
                include_usage: true,
            })
            .messages(vec![user_message(&request.text)?])
            .build()?;

        let mut gradual_response = String::new();
        let mut usage = Usage::default();

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
                            action_per_chunk(delta);
                            gradual_response.push_str(delta);
                        }

                        if let Some(u) = chunk.usage {
                            usage =
                                Usage::new(u.prompt_tokens, u.completion_tokens, u.total_tokens);
                        }
                    }
                    Err(e) => {
                        error!("Error in stream: {e}");
                        break;
                    }
                }
            }
        });

        debug!("Response: {gradual_response}");

        Ok(LlmResponse {
            text: gradual_response,
            usage,
        })
    }

    pub fn get_chat_completion(
        &self,
        request: &LlmRequest,
    ) -> Result<LlmResponse, Box<dyn std::error::Error>> {
        self.get_chat_completion_streaming(request, |_| {})
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
