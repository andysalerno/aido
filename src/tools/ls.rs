use serde_json::Value;

use crate::tools::{
    Arg, ArgType, Tool, ToolDefinition, ToolDefinitionBuilder, ToolInput,
};

pub struct Ls {
    definition: ToolDefinition,
}

impl Ls {
    pub fn new() -> Self {
        let definition = ToolDefinitionBuilder::new("ls")
            .description("List directory contents")
            .arg(
                Arg::new("args")
                    .description("The input arguments for ls. Example: -alh")
                    .kind(ArgType::String),
            )
            .build();
        Self { definition }
    }
}

impl Tool for Ls {
    fn execute(
        &self,
        input: ToolInput,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let maybe_input = input.get("args").and_then(Value::as_str);

        let mut command = std::process::Command::new("/bin/ls");

        if let Some(args) = maybe_input {
            command.arg(args);
        }

        // First, get the current working directory of this process:
        command.current_dir(std::env::current_dir()?);

        let output = command.output()?.stdout;

        Ok(String::from_utf8(output)?)
    }

    fn definition(&self) -> &ToolDefinition {
        &self.definition
    }
}
