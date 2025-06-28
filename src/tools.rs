mod ls;

use serde::Serialize;
use serde_json::{Map, Value, json};

pub use ls::*;

pub trait Tool {
    fn definition(&self) -> ToolDefinition;

    /// Executes the tool with the given input and returns a result.
    fn execute(
        &self,
        input: &str,
    ) -> Result<String, Box<dyn std::error::Error>>;
}

#[derive(Debug, Clone, Copy)]
pub enum ArgType {
    String,
    Number,
    Integer,
    Boolean,
    Object,
    Array,
}

impl ArgType {
    fn as_str(self) -> &'static str {
        match self {
            ArgType::String => "string",
            ArgType::Number => "number",
            ArgType::Integer => "integer",
            ArgType::Boolean => "boolean",
            ArgType::Object => "object",
            ArgType::Array => "array",
        }
    }
}

#[derive(Debug)]
pub struct Arg {
    name: String,
    description: String,
    kind: ArgType,
    enum_vals: Option<Vec<String>>,
    required: bool,
}

impl Arg {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: String::new(),
            kind: ArgType::String,
            enum_vals: None,
            required: false,
        }
    }
    pub fn description(mut self, text: impl Into<String>) -> Self {
        self.description = text.into();
        self
    }
    pub fn kind(mut self, kind: ArgType) -> Self {
        self.kind = kind;
        self
    }
    pub fn with_enum<I, S>(mut self, vals: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.enum_vals = Some(vals.into_iter().map(Into::into).collect());
        self
    }
    pub fn required(mut self) -> Self {
        self.required = true;
        self
    }
}

pub struct ToolDefinition {
    name: String,
    description: String,
    args: Vec<Arg>,
}

impl ToolDefinition {
    pub fn new(name: String, description: String, args: Vec<Arg>) -> Self {
        Self { name, description, args }
    }

    pub fn into_json_value(self) -> Value {
        let mut props = Map::new();
        let mut required = Vec::new();

        for a in &self.args {
            if a.required {
                required.push(a.name.clone());
            }
            let mut entry = json!({
                "type": a.kind.as_str(),
                "description": a.description,
            });
            if let Some(vals) = &a.enum_vals {
                entry
                    .as_object_mut()
                    .unwrap()
                    .insert("enum".to_string(), json!(vals));
            }
            props.insert(a.name.clone(), entry);
        }

        json!({
            "type": "function",
            "function": {
                "name": self.name,
                "description": self.description,
                "parameters": {
                    "type": "object",
                    "properties": props,
                    "required": required
                }
            }
        })
    }
}

#[derive(Debug)]
pub struct ToolDefinitionBuilder {
    name: String,
    description: String,
    args: Vec<Arg>,
}

impl ToolDefinitionBuilder {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: String::new(),
            args: Vec::new(),
        }
    }
    pub fn description(mut self, text: impl Into<String>) -> Self {
        self.description = text.into();
        self
    }
    pub fn arg(mut self, arg: Arg) -> Self {
        self.args.push(arg);
        self
    }

    /// Consumes the builder and returns a ToolDefinition
    pub fn build(self) -> ToolDefinition {
        ToolDefinition::new(self.name, self.description, self.args)
    }
}

#[derive(Serialize)]
pub struct ToolList {
    tools: Vec<Value>,
}
impl ToolList {
    pub fn new(tools: impl Into<Vec<Value>>) -> Self {
        Self { tools: tools.into() }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_schema() {
        let tool = ToolDefinitionBuilder::new("my_tool")
            .description("some test tool")
            .arg(
                Arg::new("myArgument")
                    .description("Some description of my argument")
                    .kind(ArgType::String)
                    .required(),
            )
            .arg(
                Arg::new("myOtherArgument")
                    .description("Some other argument")
                    .kind(ArgType::Number),
            )
            .build();

        let expected_json = serde_json::json!({
            "type": "function",
            "function": {
                "name": "my_tool",
                "description": "some test tool",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "myArgument": {
                            "type": "string",
                            "description": "Some description of my argument",
                        },
                        "myOtherArgument": {
                            "type": "number",
                            "description": "Some other argument"
                        }
                    },
                    "required": [
                        "myArgument"
                    ]
                }
            }
        });

        let tool_str = serde_json::to_string(&tool.into_json_value()).unwrap();
        let expected_str = serde_json::to_string(&expected_json).unwrap();

        assert_eq!(tool_str, expected_str);
    }
}
