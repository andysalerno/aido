use std::io::Write;

use log::{debug, info};

use crate::{
    config::Config,
    llm::{self, LlmRequest},
};

pub fn run(config: Config, input: &str) -> Result<(), Box<dyn std::error::Error>> {
    let llm = llm::LlmClient::new(config.model_name, config.api_key, config.api_url);

    let response =
        llm.get_chat_completion_streaming(&LlmRequest { text: input.into() }, |chunk| {
            print!("{chunk}");
        })?;

    println!();

    std::io::stdout().flush()?;

    debug!("LLM Response: {}", response.text());
    info!("{:?}", response.usage());

    Ok(())
}
