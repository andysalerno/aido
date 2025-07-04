mod ls;

pub use ls::Ls;

use core::fmt;
use std::collections::HashMap;

use serde::Serialize;
use serde_json::{Map, Value, json};

pub type ToolInput = HashMap<String, Value>;

pub trait Tool {
    fn definition(&self) -> &ToolDefinition;

    /// Executes the tool with the given input and returns a result.
    fn execute(
        &self,
        input: ToolInput,
    ) -> Result<String, Box<dyn std::error::Error>>;
}

impl fmt::Debug for dyn Tool {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Tool")
            .field("name", &self.definition().name)
            .field("description", &self.definition().description)
            .finish()
    }
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
            Self::String => "string",
            Self::Number => "number",
            Self::Integer => "integer",
            Self::Boolean => "boolean",
            Self::Object => "object",
            Self::Array => "array",
        }
    }
}

#[derive(Debug, Clone)]
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

#[derive(Debug, Clone)]
pub struct ToolDefinition {
    name: String,
    description: String,
    args: Vec<Arg>,
}

impl ToolDefinition {
    pub fn new(name: String, description: String, args: Vec<Arg>) -> Self {
        Self { name, description, args }
    }

    pub fn json_value(&self) -> Value {
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

        // json!({
        //     "type": "function",
        //     "function": {
        //         "name": self.name,
        //         "description": self.description,
        //         "parameters": {
        //             "type": "object",
        //             "properties": props,
        //             "required": required
        //         }
        //     }
        // })
        json!({
            "type": "object",
            "properties": props,
            "required": required
        })
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn description(&self) -> &str {
        &self.description
    }

    pub fn args(&self) -> &[Arg] {
        &self.args
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

    /// Consumes the builder and returns a `ToolDefinition`
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

        let expected_json = serde_json::json!(
        {
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
        });

        let tool_str = serde_json::to_string(&tool.json_value()).unwrap();
        let expected_str = serde_json::to_string(&expected_json).unwrap();

        assert_eq!(tool_str, expected_str);
    }

    #[test]
    fn test_empty_tool_definition() {
        let tool = ToolDefinitionBuilder::new("empty_tool")
            .description("A tool with no arguments")
            .build();

        let expected_json = serde_json::json!(
        {
            "type": "object",
            "properties": {},
            "required": []
        });

        let actual_json = tool.json_value();
        assert_eq!(actual_json, expected_json);
    }

    #[test]
    fn test_tool_with_all_arg_types() {
        let tool = ToolDefinitionBuilder::new("type_test")
            .description("Testing all argument types")
            .arg(
                Arg::new("string_arg")
                    .kind(ArgType::String)
                    .description("A string"),
            )
            .arg(
                Arg::new("number_arg")
                    .kind(ArgType::Number)
                    .description("A number"),
            )
            .arg(
                Arg::new("integer_arg")
                    .kind(ArgType::Integer)
                    .description("An integer"),
            )
            .arg(
                Arg::new("boolean_arg")
                    .kind(ArgType::Boolean)
                    .description("A boolean"),
            )
            .arg(
                Arg::new("object_arg")
                    .kind(ArgType::Object)
                    .description("An object"),
            )
            .arg(
                Arg::new("array_arg")
                    .kind(ArgType::Array)
                    .description("An array"),
            )
            .build();

        let json = tool.json_value();
        let properties = &json["properties"];

        assert_eq!(properties["string_arg"]["type"], "string");
        assert_eq!(properties["number_arg"]["type"], "number");
        assert_eq!(properties["integer_arg"]["type"], "integer");
        assert_eq!(properties["boolean_arg"]["type"], "boolean");
        assert_eq!(properties["object_arg"]["type"], "object");
        assert_eq!(properties["array_arg"]["type"], "array");
    }

    #[test]
    fn test_tool_with_enum_values() {
        let tool = ToolDefinitionBuilder::new("enum_tool")
            .description("Tool with enum arguments")
            .arg(
                Arg::new("color")
                    .description("Choose a color")
                    .kind(ArgType::String)
                    .with_enum(["red", "green", "blue"])
                    .required(),
            )
            .arg(
                Arg::new("size")
                    .description("Choose a size")
                    .kind(ArgType::String)
                    .with_enum(vec![
                        "small".to_string(),
                        "medium".to_string(),
                        "large".to_string(),
                    ]),
            )
            .build();

        let json = tool.json_value();
        let properties = &json["properties"];

        assert_eq!(
            properties["color"]["enum"],
            serde_json::json!(["red", "green", "blue"])
        );
        assert_eq!(
            properties["size"]["enum"],
            serde_json::json!(["small", "medium", "large"])
        );
        assert_eq!(json["required"], serde_json::json!(["color"]));
    }

    #[test]
    fn test_tool_with_multiple_required_args() {
        let tool = ToolDefinitionBuilder::new("multi_required")
            .description("Tool with multiple required arguments")
            .arg(Arg::new("first").description("First arg").required())
            .arg(Arg::new("second").description("Second arg"))
            .arg(Arg::new("third").description("Third arg").required())
            .arg(Arg::new("fourth").description("Fourth arg").required())
            .build();

        let json = tool.json_value();
        let required = &json["required"];

        assert!(
            required.as_array().unwrap().contains(&serde_json::json!("first"))
        );
        assert!(
            required.as_array().unwrap().contains(&serde_json::json!("third"))
        );
        assert!(
            required
                .as_array()
                .unwrap()
                .contains(&serde_json::json!("fourth"))
        );
        assert!(
            !required
                .as_array()
                .unwrap()
                .contains(&serde_json::json!("second"))
        );
        assert_eq!(required.as_array().unwrap().len(), 3);
    }

    #[test]
    fn test_arg_builder_methods() {
        let arg = Arg::new("test_arg")
            .description("Test description")
            .kind(ArgType::Integer)
            .with_enum([1, 2, 3].map(|n| n.to_string()))
            .required();

        assert_eq!(arg.name, "test_arg");
        assert_eq!(arg.description, "Test description");
        assert!(matches!(arg.kind, ArgType::Integer));
        assert_eq!(
            arg.enum_vals,
            Some(vec!["1".to_string(), "2".to_string(), "3".to_string()])
        );
        assert!(arg.required);
    }

    #[test]
    fn test_arg_default_values() {
        let arg = Arg::new("default_arg");

        assert_eq!(arg.name, "default_arg");
        assert_eq!(arg.description, "");
        assert!(matches!(arg.kind, ArgType::String));
        assert_eq!(arg.enum_vals, None);
        assert!(!arg.required);
    }

    #[test]
    fn test_tool_definition_builder_chaining() {
        let tool = ToolDefinitionBuilder::new("chained_tool")
            .description("First description")
            .description("Updated description") // Should overwrite
            .arg(Arg::new("arg1").description("First arg"))
            .arg(Arg::new("arg2").description("Second arg"))
            .build();

        let json = tool.json_value();

        assert!(json["properties"]["arg1"].is_object());
        assert!(json["properties"]["arg2"].is_object());
    }

    #[test]
    fn test_tool_list_creation() {
        let tool1 = ToolDefinitionBuilder::new("tool1")
            .description("First tool")
            .build()
            .json_value();

        let tool2 = ToolDefinitionBuilder::new("tool2")
            .description("Second tool")
            .arg(Arg::new("param").description("A parameter"))
            .build()
            .json_value();

        let tool_list = ToolList::new(vec![tool1.clone(), tool2.clone()]);

        assert_eq!(tool_list.tools.len(), 2);
        assert_eq!(tool_list.tools[0], tool1);
        assert_eq!(tool_list.tools[1], tool2);
    }

    #[test]
    fn test_tool_definition_direct_creation() {
        let args = vec![
            Arg::new("direct_arg")
                .description("Directly created argument")
                .kind(ArgType::Boolean)
                .required(),
        ];

        let tool = ToolDefinition::new(
            "direct_tool".to_string(),
            "Directly created tool".to_string(),
            args,
        );

        let json = tool.json_value();

        assert_eq!(json["properties"]["direct_arg"]["type"], "boolean");
        assert_eq!(json["required"], serde_json::json!(["direct_arg"]));
    }

    #[test]
    fn test_arg_type_string_conversion() {
        assert_eq!(ArgType::String.as_str(), "string");
        assert_eq!(ArgType::Number.as_str(), "number");
        assert_eq!(ArgType::Integer.as_str(), "integer");
        assert_eq!(ArgType::Boolean.as_str(), "boolean");
        assert_eq!(ArgType::Object.as_str(), "object");
        assert_eq!(ArgType::Array.as_str(), "array");
    }

    #[test]
    fn test_complex_tool_with_mixed_args() {
        let tool = ToolDefinitionBuilder::new("complex_tool")
            .description("A complex tool demonstrating various features")
            .arg(
                Arg::new("required_string")
                    .description("A required string parameter")
                    .kind(ArgType::String)
                    .required(),
            )
            .arg(
                Arg::new("optional_enum")
                    .description("An optional enum parameter")
                    .kind(ArgType::String)
                    .with_enum(["option1", "option2", "option3"]),
            )
            .arg(
                Arg::new("required_number")
                    .description("A required number parameter")
                    .kind(ArgType::Number)
                    .required(),
            )
            .arg(
                Arg::new("optional_boolean")
                    .description("An optional boolean parameter")
                    .kind(ArgType::Boolean),
            )
            .build();

        let json = tool.json_value();
        let required = json["required"].as_array().unwrap();

        // Check required arguments
        assert_eq!(required.len(), 2);
        assert!(required.contains(&serde_json::json!("required_string")));
        assert!(required.contains(&serde_json::json!("required_number")));

        // Check property types and descriptions
        assert_eq!(json["properties"]["required_string"]["type"], "string");
        assert_eq!(
            json["properties"]["required_string"]["description"],
            "A required string parameter"
        );

        assert_eq!(json["properties"]["optional_enum"]["type"], "string");
        assert_eq!(
            json["properties"]["optional_enum"]["enum"],
            serde_json::json!(["option1", "option2", "option3"])
        );

        assert_eq!(json["properties"]["required_number"]["type"], "number");
        assert_eq!(json["properties"]["optional_boolean"]["type"], "boolean");
    }
}
