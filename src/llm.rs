use async_openai::{
    Client,
    config::OpenAIConfig,
    types::{
        ChatChoiceStream, ChatCompletionMessageToolCall,
        ChatCompletionRequestAssistantMessageArgs,
        ChatCompletionRequestAssistantMessageContent,
        ChatCompletionRequestMessage, ChatCompletionRequestSystemMessageArgs,
        ChatCompletionRequestSystemMessageContent,
        ChatCompletionRequestToolMessageArgs,
        ChatCompletionRequestToolMessageContent,
        ChatCompletionRequestUserMessageArgs,
        ChatCompletionRequestUserMessageContent, ChatCompletionStreamOptions,
        ChatCompletionTool, ChatCompletionToolType,
        CreateChatCompletionRequestArgs, FunctionCall, FunctionObjectArgs,
    },
};
use futures_util::StreamExt;
use log::{debug, error, trace};
use tokio::runtime::Runtime;

use crate::tools::ToolDefinition;

pub struct LlmClient {
    client: Client<OpenAIConfig>,
    model_name: String,
}

#[derive(Debug, Default)]
pub struct LlmRequest {
    messages: Vec<Message>,
    tools: Vec<ToolDefinition>,
}

impl LlmRequest {
    pub fn new(messages: Vec<Message>, tools: Vec<ToolDefinition>) -> Self {
        Self { messages, tools }
    }

    pub fn messages(&self) -> &[Message] {
        &self.messages
    }

    pub fn tools(&self) -> &[ToolDefinition] {
        &self.tools
    }
}

impl From<Message> for ChatCompletionRequestMessage {
    fn from(value: Message) -> Self {
        match value {
            Message::User(content) => {
                ChatCompletionRequestUserMessageArgs::default()
                    .content(ChatCompletionRequestUserMessageContent::Text(
                        content,
                    ))
                    .build()
                    .unwrap()
                    .into()
            }
            Message::Assistant(content, tool_calls) => {
                let converted_tools = tool_calls.map(|tools| {
                    tools
                        .into_iter()
                        .map(std::convert::Into::into)
                        .collect::<Vec<ChatCompletionMessageToolCall>>()
                });
                ChatCompletionRequestAssistantMessageArgs::default()
                    .content(
                        ChatCompletionRequestAssistantMessageContent::Text(
                            content,
                        ),
                    )
                    .tool_calls(converted_tools.unwrap_or_default())
                    .build()
                    .unwrap()
                    .into()
            }
            Message::System(content) => {
                ChatCompletionRequestSystemMessageArgs::default()
                    .content(ChatCompletionRequestSystemMessageContent::Text(
                        content,
                    ))
                    .build()
                    .unwrap()
                    .into()
            }
            Message::Tool { content, id } => {
                ChatCompletionRequestToolMessageArgs::default()
                    .content(ChatCompletionRequestToolMessageContent::Text(
                        content,
                    ))
                    .tool_call_id(id)
                    .build()
                    .unwrap()
                    .into()
            }
        }
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    User(String),
    Assistant(String, Option<Vec<ToolCall>>),
    System(String),
    Tool { content: String, id: String },
}

#[derive(Debug, Clone, Default)]
pub struct LlmResponse {
    text: String,
    usage: Usage,
    tool_calls: Vec<ToolCall>,
}

impl LlmResponse {
    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn usage(&self) -> &Usage {
        &self.usage
    }

    pub fn tool_calls(&self) -> &[ToolCall] {
        &self.tool_calls
    }
}

fn llm_response_from_stream(
    stream: &ChatChoiceStream,
    usage: Usage,
) -> LlmResponse {
    let mut text = String::new();
    let mut tool_calls = Vec::new();

    if let Some(text_output) = &stream.delta.content {
        text.push_str(text_output);
    }

    if let Some(tool_calls_vec) = &stream.delta.tool_calls {
        for tool_call in tool_calls_vec {
            let id = tool_call.id.clone().unwrap();
            let function = tool_call.function.as_ref().unwrap();
            let name = function.name.clone().unwrap();
            let arguments = function.arguments.clone().unwrap_or_default();

            tool_calls.push(ToolCall { id, name, arguments });
        }
    }

    LlmResponse { text, usage, tool_calls }
}

#[derive(Debug, Clone, Default)]
pub struct ToolCall {
    id: String,
    name: String,
    arguments: String,
}

impl ToolCall {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn arguments(&self) -> &str {
        &self.arguments
    }

    pub fn id(&self) -> &str {
        &self.id
    }
}

impl From<ToolCall> for ChatCompletionMessageToolCall {
    fn from(tool_call: ToolCall) -> Self {
        Self {
            id: tool_call.id,
            r#type: ChatCompletionToolType::Function,
            function: FunctionCall {
                name: tool_call.name,
                arguments: tool_call.arguments,
            },
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct Usage {
    prompt_tokens: u32,
    completion_tokens: u32,
    total_tokens: u32,
}

impl Usage {
    pub fn new(
        prompt_tokens: u32,
        completion_tokens: u32,
        total_tokens: u32,
    ) -> Self {
        Self { prompt_tokens, completion_tokens, total_tokens }
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
        let config =
            OpenAIConfig::new().with_api_key(api_key).with_api_base(base_uri);

        let client = Client::with_config(config);
        let model_name = model_name.into();

        Self { client, model_name }
    }

    pub fn get_chat_completion_streaming(
        &self,
        request: &LlmRequest,
        mut action_per_chunk: impl FnMut(&str),
    ) -> Result<LlmResponse, Box<dyn std::error::Error>> {
        let tools = request
            .tools
            .iter()
            .map(make_tool)
            .collect::<Vec<ChatCompletionTool>>();

        let messages = request
            .messages()
            .iter()
            .cloned()
            .map(std::convert::Into::into)
            .collect::<Vec<ChatCompletionRequestMessage>>();

        let request = CreateChatCompletionRequestArgs::default()
            .model(&self.model_name)
            .temperature(0.7)
            .tools(tools)
            .stream(true)
            .stream_options(ChatCompletionStreamOptions {
                include_usage: true,
            })
            .messages(messages)
            .build()?;

        if log::log_enabled!(log::Level::Debug) {
            let json = serde_json::to_string(&request)?;
            debug!("{json}");
        }

        let mut usage = Usage::default();
        let mut agg: Option<ChatChoiceStream> = None;

        RT.block_on(async {
            let mut stream =
                self.client.chat().create_stream(request).await.unwrap();

            while let Some(event) = stream.next().await {
                match event {
                    Ok(chunk) => {
                        trace!("Received chunk: {chunk:?}");

                        let choice = chunk.choices.first().unwrap();

                        if let Some(agg_chunk) = &mut agg {
                            aggregate(agg_chunk, choice);
                        } else {
                            agg = Some(choice.clone());
                        }

                        if let Some(content) = &choice.delta.content {
                            action_per_chunk(content);
                        }

                        debug!("{}", serde_json::to_string(&agg).unwrap());

                        if let Some(u) = chunk.usage {
                            usage = Usage::new(
                                u.prompt_tokens,
                                u.completion_tokens,
                                u.total_tokens,
                            );
                        }
                    }
                    Err(e) => {
                        error!("Error in stream: {e}");
                        break;
                    }
                }
            }
        });

        Ok(llm_response_from_stream(&agg.unwrap(), usage))
    }

    pub fn get_chat_completion(
        &self,
        request: &LlmRequest,
    ) -> Result<LlmResponse, Box<dyn std::error::Error>> {
        self.get_chat_completion_streaming(request, |_| {})
    }
}

fn aggregate(update_to: &mut ChatChoiceStream, from: &ChatChoiceStream) {
    // There must be a better way to do this...
    if let Some(delta) = &from.delta.content {
        if let Some(content) = &mut update_to.delta.content {
            content.push_str(delta);
        }
    }

    if let Some(tool_calls) = &from.delta.tool_calls {
        if update_to.delta.tool_calls.is_none() {
            update_to.delta.tool_calls = Some(vec![]);
        }

        for tool_call in tool_calls {
            let index = tool_call.index as usize;

            // God forgive me for what I am about to do...
            if index >= update_to.delta.tool_calls.as_ref().unwrap().len() {
                update_to
                    .delta
                    .tool_calls
                    .as_mut()
                    .unwrap()
                    .push(tool_call.clone());
            } else {
                let function_args =
                    update_to.delta.tool_calls.as_mut().unwrap()[index]
                        .function
                        .as_mut()
                        .unwrap()
                        .arguments
                        .as_mut()
                        .unwrap();

                function_args.push_str(
                    tool_call
                        .function
                        .as_ref()
                        .unwrap()
                        .arguments
                        .as_ref()
                        .unwrap(),
                );
            }
        }
    }

    if let Some(finish_reason) = &from.finish_reason {
        update_to.finish_reason = Some(*finish_reason);
    }
}

fn make_tool(tool: &ToolDefinition) -> ChatCompletionTool {
    let name = tool.name().to_owned();
    let description = tool.description().to_owned();
    let tool_json = tool.json_value();

    ChatCompletionTool {
        r#type: ChatCompletionToolType::Function,
        function: FunctionObjectArgs::default()
            .name(name)
            .description(description)
            .parameters(tool_json)
            .build()
            .unwrap(),
    }
}
