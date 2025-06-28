use crate::tools::Tool;

pub struct Ls;

impl Tool for Ls {
    fn execute(
        &self,
        input: &str,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let output =
            std::process::Command::new("ls").arg(input).output()?.stdout;

        Ok(String::from_utf8(output)?)
    }

    fn definition(&self) -> super::ToolDefinition {
        todo!()
    }
}
