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
        target.function = Some(FunctionCallStream {
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
                target_tool_calls.push(ChatCompletionMessageToolCallChunk {
                    index: u32::try_from(target_tool_calls.len()).unwrap_or(0),
                    id: None,
                    r#type: None,
                    function: None,
                });
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::{Arg, ArgType, ToolDefinition};
    use async_openai::types::{
        ChatCompletionStreamResponseDelta, FinishReason, FunctionCallStream,
    };
    use std::error::Error;

    #[test]
    fn test_llm_error_display() {
        let api_error = LlmError::ApiError(
            async_openai::error::OpenAIError::InvalidArgument(
                "test".to_string(),
            ),
        );
        assert!(api_error.to_string().contains("API error"));

        let serialization_error = LlmError::SerializationError(
            serde_json::from_str::<serde_json::Value>("invalid json")
                .unwrap_err(),
        );
        assert!(
            serialization_error.to_string().contains("Serialization error")
        );

        let invalid_response =
            LlmError::InvalidResponse("bad response".to_string());
        assert_eq!(
            invalid_response.to_string(),
            "Invalid response: bad response"
        );

        let missing_data = LlmError::MissingData("missing field".to_string());
        assert_eq!(
            missing_data.to_string(),
            "Missing required data: missing field"
        );
    }

    #[test]
    fn test_llm_error_source() {
        let api_error = LlmError::ApiError(
            async_openai::error::OpenAIError::InvalidArgument(
                "test".to_string(),
            ),
        );
        assert!(api_error.source().is_some());

        let serialization_error = LlmError::SerializationError(
            serde_json::from_str::<serde_json::Value>("invalid json")
                .unwrap_err(),
        );
        assert!(serialization_error.source().is_some());

        let invalid_response =
            LlmError::InvalidResponse("bad response".to_string());
        assert!(invalid_response.source().is_none());

        let missing_data = LlmError::MissingData("missing field".to_string());
        assert!(missing_data.source().is_none());
    }

    #[test]
    fn test_llm_error_from_conversions() {
        let openai_error = async_openai::error::OpenAIError::InvalidArgument(
            "test".to_string(),
        );
        let llm_error: LlmError = openai_error.into();
        assert!(matches!(llm_error, LlmError::ApiError(_)));

        assert!(matches!(llm_error, LlmError::ApiError(_)));

        let json_error =
            serde_json::from_str::<serde_json::Value>("invalid json")
                .unwrap_err();
        let llm_error: LlmError = json_error.into();
        assert!(matches!(llm_error, LlmError::SerializationError(_)));
    }

    #[test]
    fn test_usage_creation_and_getters() {
        let usage = Usage::new(100, 50, 150);
        assert_eq!(usage.prompt_tokens(), 100);
        assert_eq!(usage.completion_tokens(), 50);
        assert_eq!(usage.total_tokens(), 150);
    }

    #[test]
    fn test_usage_default() {
        let usage = Usage::default();
        assert_eq!(usage.prompt_tokens(), 0);
        assert_eq!(usage.completion_tokens(), 0);
        assert_eq!(usage.total_tokens(), 0);
    }

    #[test]
    fn test_tool_call_creation_and_getters() {
        let tool_call = ToolCall {
            id: "call_123".to_string(),
            name: "test_tool".to_string(),
            arguments: r#"{"param": "value"}"#.to_string(),
        };

        assert_eq!(tool_call.id(), "call_123");
        assert_eq!(tool_call.name(), "test_tool");
        assert_eq!(tool_call.arguments(), r#"{"param": "value"}"#);
    }

    #[test]
    fn test_tool_call_default() {
        let tool_call = ToolCall::default();
        assert_eq!(tool_call.id(), "");
        assert_eq!(tool_call.name(), "");
        assert_eq!(tool_call.arguments(), "");
    }

    #[test]
    fn test_tool_call_to_chat_completion_message_tool_call() {
        let tool_call = ToolCall {
            id: "call_123".to_string(),
            name: "test_tool".to_string(),
            arguments: r#"{"param": "value"}"#.to_string(),
        };

        let completion_tool_call: ChatCompletionMessageToolCall =
            tool_call.into();
        assert_eq!(completion_tool_call.id, "call_123");
        assert_eq!(completion_tool_call.function.name, "test_tool");
        assert_eq!(
            completion_tool_call.function.arguments,
            r#"{"param": "value"}"#
        );
        assert_eq!(
            completion_tool_call.r#type,
            ChatCompletionToolType::Function
        );
    }

    #[test]
    fn test_message_user_conversion() {
        let message = Message::User("Hello, world!".to_string());
        let completion_message: ChatCompletionRequestMessage = message.into();

        match completion_message {
            ChatCompletionRequestMessage::User(user_msg) => {
                match user_msg.content {
                    ChatCompletionRequestUserMessageContent::Text(text) => {
                        assert_eq!(text, "Hello, world!");
                    }
                    _ => panic!("Expected text content"),
                }
            }
            _ => panic!("Expected user message"),
        }
    }

    #[test]
    fn test_message_assistant_conversion_without_tools() {
        let message = Message::Assistant("Hi there!".to_string(), None);
        let completion_message: ChatCompletionRequestMessage = message.into();

        match completion_message {
            ChatCompletionRequestMessage::Assistant(assistant_msg) => {
                match assistant_msg.content {
                    Some(
                        ChatCompletionRequestAssistantMessageContent::Text(
                            text,
                        ),
                    ) => {
                        assert_eq!(text, "Hi there!");
                    }
                    _ => panic!("Expected text content"),
                }
                assert!(assistant_msg.tool_calls.as_ref().unwrap().is_empty());
            }
            _ => panic!("Expected assistant message"),
        }
    }

    #[test]
    fn test_message_assistant_conversion_with_tools() {
        let tool_calls = vec![ToolCall {
            id: "call_123".to_string(),
            name: "test_tool".to_string(),
            arguments: r#"{"param": "value"}"#.to_string(),
        }];
        let message =
            Message::Assistant("Using tool".to_string(), Some(tool_calls));
        let completion_message: ChatCompletionRequestMessage = message.into();

        match completion_message {
            ChatCompletionRequestMessage::Assistant(assistant_msg) => {
                assert_eq!(
                    assistant_msg.tool_calls.as_ref().unwrap().len(),
                    1
                );
                assert_eq!(
                    assistant_msg.tool_calls.as_ref().unwrap()[0].id,
                    "call_123"
                );
            }
            _ => panic!("Expected assistant message"),
        }
    }

    #[test]
    fn test_message_system_conversion() {
        let message =
            Message::System("You are a helpful assistant".to_string());
        let completion_message: ChatCompletionRequestMessage = message.into();

        match completion_message {
            ChatCompletionRequestMessage::System(system_msg) => {
                match system_msg.content {
                    ChatCompletionRequestSystemMessageContent::Text(text) => {
                        assert_eq!(text, "You are a helpful assistant");
                    }
                    _ => panic!("Expected text content"),
                }
            }
            _ => panic!("Expected system message"),
        }
    }

    #[test]
    fn test_message_tool_conversion() {
        let message = Message::Tool {
            content: "Tool result".to_string(),
            id: "call_123".to_string(),
        };
        let completion_message: ChatCompletionRequestMessage = message.into();

        match completion_message {
            ChatCompletionRequestMessage::Tool(tool_msg) => {
                match tool_msg.content {
                    ChatCompletionRequestToolMessageContent::Text(text) => {
                        assert_eq!(text, "Tool result");
                    }
                    _ => panic!("Expected text content"),
                }
                assert_eq!(tool_msg.tool_call_id, "call_123");
            }
            _ => panic!("Expected tool message"),
        }
    }

    #[test]
    fn test_llm_request_creation() {
        let messages = vec![
            Message::System("You are helpful".to_string()),
            Message::User("Hello".to_string()),
        ];
        let tools = vec![]; // Empty tools for simplicity
        let request = LlmRequest::new(messages, tools);

        assert_eq!(request.messages().len(), 2);
        assert_eq!(request.tools().len(), 0);
    }

    #[test]
    fn test_llm_response_creation_and_getters() {
        let usage = Usage::new(100, 50, 150);
        let tool_calls = vec![ToolCall {
            id: "call_123".to_string(),
            name: "test_tool".to_string(),
            arguments: r#"{"param": "value"}"#.to_string(),
        }];
        let response = LlmResponse {
            text: "Hello, world!".to_string(),
            usage,
            tool_calls,
        };

        assert_eq!(response.text(), "Hello, world!");
        assert_eq!(response.usage().total_tokens(), 150);
        assert_eq!(response.tool_calls().len(), 1);
        assert_eq!(response.tool_calls()[0].id(), "call_123");
    }

    #[test]
    fn test_llm_response_default() {
        let response = LlmResponse::default();
        assert_eq!(response.text(), "");
        assert_eq!(response.usage().total_tokens(), 0);
        assert_eq!(response.tool_calls().len(), 0);
    }

    #[test]
    fn test_llm_client_creation() {
        let client = LlmClient::new(
            "gpt-4",
            "test-api-key",
            "https://api.openai.com/v1",
        );
        assert_eq!(client.model_name, "gpt-4");
        assert_eq!(client.temperature, 0.7);
    }

    #[test]
    fn test_llm_client_with_temperature() {
        let client = LlmClient::new(
            "gpt-4",
            "test-api-key",
            "https://api.openai.com/v1",
        )
        .with_temperature(0.3);

        assert_eq!(client.temperature, 0.3);
    }

    #[test]
    fn test_merge_function_calls_with_new_target() {
        let mut target = ChatCompletionMessageToolCallChunk {
            index: 0,
            id: Some("call_123".to_string()),
            r#type: Some(ChatCompletionToolType::Function),
            function: None,
        };

        let source = FunctionCallStream {
            name: Some("test_tool".to_string()),
            arguments: Some(r#"{"param": "value"}"#.to_string()),
        };

        merge_function_calls(&mut target, &source);

        assert!(target.function.is_some());
        let function = target.function.as_ref().unwrap();
        assert_eq!(function.name, Some("test_tool".to_string()));
        assert_eq!(
            function.arguments,
            Some(r#"{"param": "value"}"#.to_string())
        );
    }

    #[test]
    fn test_merge_function_calls_with_existing_target() {
        let mut target = ChatCompletionMessageToolCallChunk {
            index: 0,
            id: Some("call_123".to_string()),
            r#type: Some(ChatCompletionToolType::Function),
            function: Some(FunctionCallStream {
                name: Some("test_tool".to_string()),
                arguments: Some(r#"{"param": "#.to_string()),
            }),
        };

        let source = FunctionCallStream {
            name: None,
            arguments: Some(r#""value"}"#.to_string()),
        };

        merge_function_calls(&mut target, &source);

        let function = target.function.as_ref().unwrap();
        assert_eq!(function.name, Some("test_tool".to_string()));
        assert_eq!(
            function.arguments,
            Some(r#"{"param": "value"}"#.to_string())
        );
    }

    #[test]
    fn test_merge_function_calls_with_missing_name() {
        let mut target = ChatCompletionMessageToolCallChunk {
            index: 0,
            id: Some("call_123".to_string()),
            r#type: Some(ChatCompletionToolType::Function),
            function: Some(FunctionCallStream {
                name: None,
                arguments: Some(r#"{"param": "value"}"#.to_string()),
            }),
        };

        let source = FunctionCallStream {
            name: Some("test_tool".to_string()),
            arguments: None,
        };

        merge_function_calls(&mut target, &source);

        let function = target.function.as_ref().unwrap();
        assert_eq!(function.name, Some("test_tool".to_string()));
        assert_eq!(
            function.arguments,
            Some(r#"{"param": "value"}"#.to_string())
        );
    }

    #[test]
    fn test_merge_stream_content_with_new_content() {
        let mut target = ChatChoiceStream {
            index: 0,
            delta: ChatCompletionStreamResponseDelta {
                content: None,
                tool_calls: None,
                role: None,
                function_call: None,
                refusal: None,
            },
            finish_reason: None,
            logprobs: None,
        };

        let source = ChatChoiceStream {
            index: 0,
            delta: ChatCompletionStreamResponseDelta {
                content: Some("Hello, ".to_string()),
                tool_calls: None,
                role: None,
                function_call: None,
                refusal: None,
            },
            finish_reason: None,
            logprobs: None,
        };

        merge_stream_content(&mut target, &source);

        assert_eq!(target.delta.content, Some("Hello, ".to_string()));
    }

    #[test]
    fn test_merge_stream_content_with_existing_content() {
        let mut target = ChatChoiceStream {
            index: 0,
            delta: ChatCompletionStreamResponseDelta {
                content: Some("Hello, ".to_string()),
                tool_calls: None,
                role: None,
                function_call: None,
                refusal: None,
            },
            finish_reason: None,
            logprobs: None,
        };

        let source = ChatChoiceStream {
            index: 0,
            delta: ChatCompletionStreamResponseDelta {
                content: Some("world!".to_string()),
                tool_calls: None,
                role: None,
                function_call: None,
                refusal: None,
            },
            finish_reason: None,
            logprobs: None,
        };

        merge_stream_content(&mut target, &source);

        assert_eq!(target.delta.content, Some("Hello, world!".to_string()));
    }

    #[test]
    fn test_merge_stream_chunks_with_finish_reason() {
        let mut target = ChatChoiceStream {
            index: 0,
            delta: ChatCompletionStreamResponseDelta {
                content: Some("Hello".to_string()),
                tool_calls: None,
                role: None,
                function_call: None,
                refusal: None,
            },
            finish_reason: None,
            logprobs: None,
        };

        let source = ChatChoiceStream {
            index: 0,
            delta: ChatCompletionStreamResponseDelta {
                content: Some(" world!".to_string()),
                tool_calls: None,
                role: None,
                function_call: None,
                refusal: None,
            },
            finish_reason: Some(FinishReason::Stop),
            logprobs: None,
        };

        merge_stream_chunks(&mut target, &source);

        assert_eq!(target.delta.content, Some("Hello world!".to_string()));
        assert_eq!(target.finish_reason, Some(FinishReason::Stop));
    }

    #[test]
    fn test_create_response_from_stream_with_content() {
        let stream = create_test_chat_choice_stream(
            0,
            Some("Hello, world!".to_string()),
            None,
            Some(FinishReason::Stop),
        );

        let usage = Usage::new(100, 50, 150);
        let response = create_response_from_stream(&stream, usage);

        assert_eq!(response.text(), "Hello, world!");
        assert_eq!(response.usage().total_tokens(), 150);
        assert_eq!(response.tool_calls().len(), 0);
    }

    #[test]
    fn test_create_response_from_stream_with_tool_calls() {
        let tool_calls = vec![ChatCompletionMessageToolCallChunk {
            index: 0,
            id: Some("call_123".to_string()),
            r#type: Some(ChatCompletionToolType::Function),
            function: Some(FunctionCallStream {
                name: Some("test_tool".to_string()),
                arguments: Some(r#"{"param": "value"}"#.to_string()),
            }),
        }];

        let stream = create_test_chat_choice_stream(
            0,
            None,
            Some(tool_calls),
            Some(FinishReason::ToolCalls),
        );

        let usage = Usage::new(100, 50, 150);
        let response = create_response_from_stream(&stream, usage);

        assert_eq!(response.text(), "");
        assert_eq!(response.tool_calls().len(), 1);
        assert_eq!(response.tool_calls()[0].id(), "call_123");
        assert_eq!(response.tool_calls()[0].name(), "test_tool");
        assert_eq!(
            response.tool_calls()[0].arguments(),
            r#"{"param": "value"}"#
        );
    }

    #[test]
    fn test_create_chat_completion_tool() {
        let args = vec![
            Arg::new("param1")
                .description("A string parameter")
                .kind(ArgType::String)
                .required(),
            Arg::new("param2")
                .description("A number parameter")
                .kind(ArgType::Number),
        ];

        let tool_def = ToolDefinition::new(
            "test_tool".to_string(),
            "A test tool".to_string(),
            args,
        );

        let chat_completion_tool = create_chat_completion_tool(&tool_def);

        assert_eq!(
            chat_completion_tool.r#type,
            ChatCompletionToolType::Function
        );
        assert_eq!(chat_completion_tool.function.name, "test_tool");
        assert_eq!(
            chat_completion_tool.function.description,
            Some("A test tool".to_string())
        );
        assert!(
            chat_completion_tool
                .function
                .parameters
                .as_ref()
                .is_some_and(serde_json::Value::is_object)
        );
    }

    #[test]
    fn test_merge_tool_calls_with_new_index() {
        let mut target = create_test_chat_choice_stream(0, None, None, None);

        let source_tool_calls = vec![ChatCompletionMessageToolCallChunk {
            index: 0,
            id: Some("call_123".to_string()),
            r#type: Some(ChatCompletionToolType::Function),
            function: Some(FunctionCallStream {
                name: Some("test_tool".to_string()),
                arguments: Some(r#"{"param": "value"}"#.to_string()),
            }),
        }];

        let source = create_test_chat_choice_stream(
            0,
            None,
            Some(source_tool_calls),
            None,
        );

        merge_tool_calls(&mut target, &source);

        assert!(target.delta.tool_calls.is_some());
        let tool_calls = target.delta.tool_calls.as_ref().unwrap();
        assert_eq!(tool_calls.len(), 1);
        assert_eq!(tool_calls[0].id, Some("call_123".to_string()));
    }

    #[test]
    fn test_merge_tool_calls_with_existing_index() {
        let existing_tool_calls = vec![ChatCompletionMessageToolCallChunk {
            index: 0,
            id: Some("call_123".to_string()),
            r#type: Some(ChatCompletionToolType::Function),
            function: Some(FunctionCallStream {
                name: Some("test_tool".to_string()),
                arguments: Some(r#"{"param": "#.to_string()),
            }),
        }];

        let mut target = create_test_chat_choice_stream(
            0,
            None,
            Some(existing_tool_calls),
            None,
        );

        let source_tool_calls = vec![ChatCompletionMessageToolCallChunk {
            index: 0,
            id: None,
            r#type: None,
            function: Some(FunctionCallStream {
                name: None,
                arguments: Some(r#""value"}"#.to_string()),
            }),
        }];

        let source = create_test_chat_choice_stream(
            0,
            None,
            Some(source_tool_calls),
            None,
        );

        merge_tool_calls(&mut target, &source);

        let tool_calls = target.delta.tool_calls.as_ref().unwrap();
        assert_eq!(tool_calls.len(), 1);
        assert_eq!(tool_calls[0].id, Some("call_123".to_string()));

        let function = tool_calls[0].function.as_ref().unwrap();
        assert_eq!(
            function.arguments,
            Some(r#"{"param": "value"}"#.to_string())
        );
    }

    #[test]
    fn test_merge_tool_calls_with_higher_index() {
        let mut target = create_test_chat_choice_stream(0, None, None, None);

        let source_tool_calls = vec![ChatCompletionMessageToolCallChunk {
            index: 2, // Higher index that requires extending the vector
            id: Some("call_456".to_string()),
            r#type: Some(ChatCompletionToolType::Function),
            function: Some(FunctionCallStream {
                name: Some("another_tool".to_string()),
                arguments: Some(r#"{"other": "param"}"#.to_string()),
            }),
        }];

        let source = create_test_chat_choice_stream(
            0,
            None,
            Some(source_tool_calls),
            None,
        );

        merge_tool_calls(&mut target, &source);

        let tool_calls = target.delta.tool_calls.as_ref().unwrap();
        assert_eq!(tool_calls.len(), 3); // Should have extended to include index 2

        // Check the placeholders at indices 0 and 1
        assert_eq!(tool_calls[0].index, 0);
        assert!(tool_calls[0].id.is_none());
        assert_eq!(tool_calls[1].index, 1);
        assert!(tool_calls[1].id.is_none());

        // Check the actual tool call at index 2
        assert_eq!(tool_calls[2].index, 2);
        assert_eq!(tool_calls[2].id, Some("call_456".to_string()));
    }

    // Helper function to create a test ChatChoiceStream
    #[allow(deprecated)]
    fn create_test_chat_choice_stream(
        index: u32,
        content: Option<String>,
        tool_calls: Option<Vec<ChatCompletionMessageToolCallChunk>>,
        finish_reason: Option<FinishReason>,
    ) -> ChatChoiceStream {
        ChatChoiceStream {
            index,
            delta: ChatCompletionStreamResponseDelta {
                content,
                tool_calls,
                role: None,
                function_call: None,
                refusal: None,
            },
            finish_reason,
            logprobs: None,
        }
    }
}
