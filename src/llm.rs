use async_openai::{
    Client,
    config::OpenAIConfig,
    types::{
        ChatChoiceStream, ChatCompletionMessageToolCall,
        ChatCompletionMessageToolCallChunk,
        ChatCompletionRequestAssistantMessageArgs,
        ChatCompletionRequestAssistantMessageContent,
        ChatCompletionRequestMessage, ChatCompletionRequestSystemMessageArgs,
        ChatCompletionRequestSystemMessageContent,
        ChatCompletionRequestToolMessageArgs,
        ChatCompletionRequestToolMessageContent,
        ChatCompletionRequestUserMessageArgs,
        ChatCompletionRequestUserMessageContent, ChatCompletionStreamOptions,
        ChatCompletionTool, ChatCompletionToolType,
        CreateChatCompletionRequestArgs, FunctionCall, FunctionCallStream,
        FunctionObjectArgs,
    },
};
use futures_util::StreamExt;
use log::{debug, error, trace};
use std::fmt;
use tokio::runtime::Runtime;

use crate::tools::ToolDefinition;

/// Errors that can occur during LLM operations
#[derive(Debug)]
pub enum LlmError {
    /// API client error
    ApiError(async_openai::error::OpenAIError),
    /// JSON serialization error
    SerializationError(serde_json::Error),
    /// Invalid response from API
    InvalidResponse(String),
    /// Missing required data in response
    MissingData(String),
}

impl fmt::Display for LlmError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ApiError(e) => write!(f, "API error: {e}"),
            Self::SerializationError(e) => {
                write!(f, "Serialization error: {e}")
            }
            Self::InvalidResponse(msg) => write!(f, "Invalid response: {msg}"),
            Self::MissingData(msg) => {
                write!(f, "Missing required data: {msg}")
            }
        }
    }
}

impl std::error::Error for LlmError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::ApiError(e) => Some(e),
            Self::SerializationError(e) => Some(e),
            Self::InvalidResponse(_) | Self::MissingData(_) => None,
        }
    }
}

impl From<async_openai::error::OpenAIError> for LlmError {
    fn from(error: async_openai::error::OpenAIError) -> Self {
        Self::ApiError(error)
    }
}

impl From<serde_json::Error> for LlmError {
    fn from(error: serde_json::Error) -> Self {
        Self::SerializationError(error)
    }
}

type LlmResult<T> = Result<T, LlmError>;

/// Client for interacting with Large Language Models via OpenAI-compatible APIs
pub struct LlmClient {
    client: Client<OpenAIConfig>,
    model_name: String,
    temperature: f32,
}

/// Request configuration for LLM chat completion
#[derive(Debug, Default)]
pub struct LlmRequest {
    messages: Vec<Message>,
    tools: Vec<ToolDefinition>,
}

impl LlmRequest {
    /// Creates a new LLM request with the specified messages and tools
    pub fn new(messages: Vec<Message>, tools: Vec<ToolDefinition>) -> Self {
        Self { messages, tools }
    }

    /// Returns the messages in this request
    pub fn messages(&self) -> &[Message] {
        &self.messages
    }

    /// Returns the tools available for this request
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
                    .expect("Failed to build user message")
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
                    .expect("Failed to build assistant message")
                    .into()
            }
            Message::System(content) => {
                ChatCompletionRequestSystemMessageArgs::default()
                    .content(ChatCompletionRequestSystemMessageContent::Text(
                        content,
                    ))
                    .build()
                    .expect("Failed to build system message")
                    .into()
            }
            Message::Tool { content, id } => {
                ChatCompletionRequestToolMessageArgs::default()
                    .content(ChatCompletionRequestToolMessageContent::Text(
                        content,
                    ))
                    .tool_call_id(id)
                    .build()
                    .expect("Failed to build tool message")
                    .into()
            }
        }
    }
}

/// A message in a conversation with an LLM
#[derive(Debug, Clone)]
pub enum Message {
    /// A message from the user
    User(String),
    /// A message from the assistant, optionally with tool calls
    Assistant(String, Option<Vec<ToolCall>>),
    /// A system message providing context or instructions
    System(String),
    /// A tool response message
    Tool { content: String, id: String },
}

/// Response from an LLM completion request
#[derive(Debug, Clone, Default)]
pub struct LlmResponse {
    text: String,
    usage: Usage,
    tool_calls: Vec<ToolCall>,
}

impl LlmResponse {
    /// Returns the text content of the response
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Returns the usage statistics for this response
    pub fn usage(&self) -> &Usage {
        &self.usage
    }

    /// Returns the tool calls made in this response
    pub fn tool_calls(&self) -> &[ToolCall] {
        &self.tool_calls
    }
}

/// Converts a stream chunk into an LLM response
fn create_response_from_stream(
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
            if let (Some(id), Some(function)) =
                (&tool_call.id, &tool_call.function)
            {
                let name = function.name.clone().unwrap_or_default();
                let arguments = function.arguments.clone().unwrap_or_default();
                tool_calls.push(ToolCall { id: id.clone(), name, arguments });
            }
        }
    }

    LlmResponse { text, usage, tool_calls }
}

/// Represents a tool call made by the LLM
#[derive(Debug, Clone, Default)]
pub struct ToolCall {
    id: String,
    name: String,
    arguments: String,
}

impl ToolCall {
    /// Returns the name of the tool that was called
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the JSON arguments passed to the tool
    pub fn arguments(&self) -> &str {
        &self.arguments
    }

    /// Returns the unique identifier for this tool call
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

/// Token usage statistics for an LLM request
#[derive(Debug, Clone, Default)]
#[allow(clippy::struct_field_names)] // API response structure requires these exact names
pub struct Usage {
    prompt_tokens: u32,
    completion_tokens: u32,
    total_tokens: u32,
}

impl Usage {
    /// Creates a new Usage instance with the specified token counts
    pub fn new(
        prompt_tokens: u32,
        completion_tokens: u32,
        total_tokens: u32,
    ) -> Self {
        Self { prompt_tokens, completion_tokens, total_tokens }
    }

    /// Returns the number of tokens used in the prompt
    pub fn prompt_tokens(&self) -> u32 {
        self.prompt_tokens
    }

    /// Returns the number of tokens used in the completion
    pub fn completion_tokens(&self) -> u32 {
        self.completion_tokens
    }

    /// Returns the total number of tokens used
    pub fn total_tokens(&self) -> u32 {
        self.total_tokens
    }
}

/// Global tokio runtime for handling async operations in sync contexts
/// Uses a single-threaded runtime to minimize overhead
static TOKIO_RUNTIME: std::sync::LazyLock<Runtime> =
    std::sync::LazyLock::new(|| {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("Failed to create tokio runtime")
    });

impl LlmClient {
    /// Creates a new LLM client with the specified configuration
    pub fn new(
        model_name: impl Into<String>,
        api_key: impl Into<String>,
        base_uri: impl Into<String>,
    ) -> Self {
        let config =
            OpenAIConfig::new().with_api_key(api_key).with_api_base(base_uri);

        let client = Client::with_config(config);
        let model_name = model_name.into();

        Self {
            client,
            model_name,
            temperature: 0.7, // Default temperature
        }
    }

    /// Sets the temperature for response generation
    pub fn with_temperature(mut self, temperature: f32) -> Self {
        self.temperature = temperature;
        self
    }

    /// Creates a streaming chat completion request
    pub fn get_chat_completion_streaming(
        &self,
        request: &LlmRequest,
        mut on_chunk: impl FnMut(&str),
    ) -> LlmResult<LlmResponse> {
        let tools = request
            .tools
            .iter()
            .map(create_chat_completion_tool)
            .collect::<Vec<ChatCompletionTool>>();

        let messages = request
            .messages()
            .iter()
            .cloned()
            .map(std::convert::Into::into)
            .collect::<Vec<ChatCompletionRequestMessage>>();

        let request = CreateChatCompletionRequestArgs::default()
            .model(&self.model_name)
            .temperature(self.temperature)
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
        let mut aggregated_response: Option<ChatChoiceStream> = None;

        TOKIO_RUNTIME.block_on(async {
            let mut stream = self
                .client
                .chat()
                .create_stream(request)
                .await
                .map_err(LlmError::from)?;

            while let Some(event) = stream.next().await {
                match event {
                    Ok(chunk) => {
                        trace!("Received chunk: {chunk:?}");

                        let choice =
                            chunk.choices.first().ok_or_else(|| {
                                LlmError::InvalidResponse(
                                    "No choices in response".to_string(),
                                )
                            })?;

                        if let Some(existing_response) =
                            &mut aggregated_response
                        {
                            merge_stream_chunks(existing_response, choice);
                        } else {
                            aggregated_response = Some(choice.clone());
                        }

                        if let Some(content) = &choice.delta.content {
                            on_chunk(content);
                        }

                        debug!(
                            "{}",
                            serde_json::to_string(&aggregated_response)
                                .unwrap_or_default()
                        );

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
                        return Err(LlmError::from(e));
                    }
                }
            }
            Ok(())
        })?;

        Ok(create_response_from_stream(
            &aggregated_response.ok_or_else(|| {
                LlmError::MissingData(
                    "No response received from stream".to_string(),
                )
            })?,
            usage,
        ))
    }

    /// Creates a non-streaming chat completion request
    pub fn get_chat_completion(
        &self,
        request: &LlmRequest,
    ) -> LlmResult<LlmResponse> {
        self.get_chat_completion_streaming(request, |_| {})
    }
}

/// Merges streaming chunks into an aggregated response
fn merge_stream_chunks(
    target: &mut ChatChoiceStream,
    source: &ChatChoiceStream,
) {
    // Merge content
    merge_stream_content(target, source);

    // Merge tool calls
    merge_tool_calls(target, source);

    // Update finish reason
    if let Some(finish_reason) = &source.finish_reason {
        target.finish_reason = Some(*finish_reason);
    }
}

fn merge_stream_content(
    target: &mut ChatChoiceStream,
    source: &ChatChoiceStream,
) {
    if let Some(delta_content) = &source.delta.content {
        if let Some(existing_content) = &mut target.delta.content {
            existing_content.push_str(delta_content);
        } else {
            target.delta.content = Some(delta_content.clone());
        }
    }
}

fn merge_function_calls(
    target: &mut ChatCompletionMessageToolCallChunk,
    source: &FunctionCallStream,
) {
    // Merge function data
    if target.function.is_none() {
        target.function = Some(async_openai::types::FunctionCallStream {
            name: source.name.clone(),
            arguments: source.arguments.clone(),
        });
    } else if let Some(target_function) = &mut target.function {
        // Copy function name if it doesn't exist
        if target_function.name.is_none() && source.name.is_some() {
            target_function.name.clone_from(&source.name);
        }

        // Append function arguments
        if let Some(args) = &source.arguments {
            let target_args =
                target_function.arguments.get_or_insert_with(String::new);
            target_args.push_str(args);
        }
    }
}

fn merge_tool_calls(target: &mut ChatChoiceStream, source: &ChatChoiceStream) {
    if let Some(tool_calls) = &source.delta.tool_calls {
        let target_tool_calls =
            target.delta.tool_calls.get_or_insert_with(Vec::new);

        for tool_call in tool_calls {
            let index = tool_call.index as usize;

            // Extend the vector if needed with empty placeholders
            while index >= target_tool_calls.len() {
                target_tool_calls.push(
                    async_openai::types::ChatCompletionMessageToolCallChunk {
                        index: u32::try_from(target_tool_calls.len())
                            .unwrap_or(0),
                        id: None,
                        r#type: None,
                        function: None,
                    },
                );
            }

            // Update the existing tool call at this index
            let target_tool_call = &mut target_tool_calls[index];

            // Copy ID if it doesn't exist
            if target_tool_call.id.is_none() && tool_call.id.is_some() {
                target_tool_call.id.clone_from(&tool_call.id);
            }

            // Copy type if it doesn't exist
            if target_tool_call.r#type.is_none() && tool_call.r#type.is_some()
            {
                target_tool_call.r#type.clone_from(&tool_call.r#type);
            }

            if let Some(function) = &tool_call.function {
                merge_function_calls(target_tool_call, function);
            }
        }
    }
}

/// Converts a tool definition into a format suitable for the `OpenAI` API
fn create_chat_completion_tool(tool: &ToolDefinition) -> ChatCompletionTool {
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
            .expect("Failed to build function object"),
    }
}
