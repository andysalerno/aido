mod ls;

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

pub use ls::*;

pub type Parameters = serde_json::Value;

#[derive(Clone, Serialize, Default, Debug, Deserialize, Eq, PartialEq)]
pub struct ToolSchema {
    pub name: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameters: Option<serde_json::Value>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub strict: Option<bool>,
}

pub trait Tool {
    /// Returns the name of the tool.
    fn name(&self) -> &str;

    /// Returns a description of the tool.
    fn description(&self) -> &str;

    fn parameters(&self) -> Option<Parameters> {
        None
    }

    /// Executes the tool with the given input and returns a result.
    fn execute(
        &self,
        input: &str,
    ) -> Result<String, Box<dyn std::error::Error>>;
}

pub fn get_tool_schema(tool: &dyn Tool) -> ToolSchema {
    ToolSchema {
        name: tool.name().to_string(),
        description: Some(tool.description().to_string()),
        parameters: tool.parameters(),
        strict: None,
    }
}

pub struct ParametersBuilder {
    parameters: serde_json::Value,
}

pub enum ValueKind {
    String,
    Number,
    Bool,
    Array,
    Object,
}

impl ParametersBuilder {
    pub fn new() -> Self {
        Self { parameters: serde_json::Value::Object(serde_json::Map::new()) }
    }

    pub fn add_parameter(
        mut self,
        name: &str,
        kind: ValueKind,
        description: &str,
    ) -> Self {
        let value = match kind {
            ValueKind::String => serde_json::Value::String(String::new()),
            ValueKind::Number => {
                serde_json::Value::Number(serde_json::Number::from(0))
            }
            ValueKind::Bool => serde_json::Value::Bool(false),
            ValueKind::Array => serde_json::Value::Array(vec![]),
            ValueKind::Object => {
                serde_json::Value::Object(serde_json::Map::new())
            }
        };

        self.parameters
            .as_object_mut()
            .unwrap()
            .insert(name.to_string(), value);
        self
    }
}

// 'tools' is a list of 'functions'
// 'function' has a 'name', 'description', 'parameters'
// 'parameters' is an object with 'type': 'object', 'properties', and 'required'
// 'properties' is a map of parameter names to their definitions

#[derive(Serialize, Debug)]
struct OpenAITool {
    r#type: &'static str, // "function"
    function: OpenAIFunction,
}

impl OpenAITool {
    fn new(function: OpenAIFunction) -> Self {
        Self { r#type: "function", function }
    }
}

#[derive(Serialize, Debug)]
struct OpenAIFunction {
    name: String,
    description: String,
    parameters: OpenAIParameters,
}

impl OpenAIFunction {
    fn new(
        name: String,
        description: String,
        parameters: OpenAIParameters,
    ) -> Self {
        Self { name, description, parameters }
    }
}

#[derive(Serialize, Debug)]
struct OpenAIParameters {
    r#type: &'static str, // "object"
    properties: HashMap<String, OpenAIParameterProperty>,
    required: Vec<String>,
}

impl OpenAIParameters {
    fn new(
        properties: HashMap<String, OpenAIParameterProperty>,
        required: Vec<String>,
    ) -> Self {
        Self { r#type: "object", properties, required }
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct OpenAIParameterProperty {
    r#type: String, // "string", "number", etc.
    description: Option<String>,
    enum_values: Option<Vec<String>>, // For enum types
}

// Example of a real http request:
// curl https://api.openai.com/v1/chat/completions \
// -H "Content-Type: application/json" \
// -H "Authorization: Bearer $OPENAI_API_KEY" \
// -d '{
//   "model": "gpt-4.1",
//   "messages": [
//     {
//       "role": "user",
//       "content": "What is the weather like in Boston today?"
//     }
//   ],
//   "tools": [
//     {
//       "type": "function",
//       "function": {
//         "name": "get_current_weather",
//         "description": "Get the current weather in a given location",
//         "parameters": {
//           "type": "object",
//           "properties": {
//             "location": {
//               "type": "string",
//               "description": "The city and state, e.g. San Francisco, CA"
//             },
//             "unit": {
//               "type": "string",
//               "enum": ["celsius", "fahrenheit"]
//             }
//           },
//           "required": ["location"]
//         }
//       }
//     }
//   ],
//   "tool_choice": "auto"
// }'

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_schema() {
        let expected_json = serde_json::json!({
            "type": "object",
            "properties": {
                "myArgument": {
                    "type": "string",
                    "description": "Some description of my argument"
                }
            },
            "required": ["path"]
        });

        let my_tool = OpenAITool::new(OpenAIFunction::new(
            "myTool".into(),
            "some tool description".into(),
            OpenAIParameters::new(HashMap::new(), Vec::new()),
        ));

        let my_tool = OpenAITool {
            r#type: "function",
            function: OpenAIFunction {
                name: "ls".to_string(),
                description: "List files in the current directory".to_string(),
                parameters: OpenAIParameters::new(
                    serde_json::json!({
                        "path": {
                            "type": "string",
                            "description": "The path to list files in",
                            "default": "."
                        }
                    })
                    .as_object()
                    .unwrap()
                    .iter()
                    .map(|(k, v)| {
                        (
                            k.clone(),
                            OpenAIParameterProperty {
                                r#type: v["type"]
                                    .as_str()
                                    .unwrap()
                                    .to_string(),
                                description: v["description"]
                                    .as_str()
                                    .map(|s| s.to_string()),
                                enum_values: v["enum"].as_array().map(|a| {
                                    a.iter()
                                        .map(|v| {
                                            v.as_str().unwrap().to_string()
                                        })
                                        .collect()
                                }),
                            },
                        )
                    })
                    .collect(),
                    vec!["path".to_string()],
                ),
            },
        };

        let serialized = serde_json::to_string(&my_tool).unwrap();
    }
}
