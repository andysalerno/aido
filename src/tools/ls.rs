use serde_json::Value;

use crate::tools::{Arg, ArgType, Tool, ToolDefinitionBuilder, ToolInput};

pub struct Ls;

impl Tool for Ls {
    fn execute(
        &self,
        input: ToolInput,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let maybe_input = input.get("args").and_then(Value::as_str);

        let mut command = std::process::Command::new("ls");

        if let Some(args) = maybe_input {
            command.arg(args);
        }

        let output = command.output()?.stdout;

        Ok(String::from_utf8(output)?)
    }

    fn definition(&self) -> super::ToolDefinition {
        ToolDefinitionBuilder::new("ls")
            .description("List directory contents")
            .arg(
                Arg::new("args")
                    .description("The input arguments for ls. Example: -alh")
                    .kind(ArgType::String),
            )
            .build()
    }
}
