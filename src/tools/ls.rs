use crate::tools::Tool;

pub struct Ls;

impl Tool for Ls {
    fn name(&self) -> &str {
        "ls"
    }

    fn description(&self) -> &str {
        "List files in the current directory"
    }

    fn parameters(&self) -> Option<super::Parameters> {
        Some(serde_json::json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "The path to list files in",
                    "default": "."
                }
            },
            "required": ["path"]
        }))
    }

    fn execute(
        &self,
        input: &str,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let output =
            std::process::Command::new("ls").arg(input).output()?.stdout;

        Ok(String::from_utf8(output)?)
    }
}
