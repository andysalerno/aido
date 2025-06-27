use log::info;

use crate::{
    config::Config,
    llm::{self, LlmRequest},
};

pub(crate) fn run(config: Config, input: &str) -> Result<(), Box<dyn std::error::Error>> {
    let llm = llm::LlmClient::new(config.model_name, config.api_key, config.api_url);

    let response = llm.get_chat_completion(&LlmRequest { text: input.into() })?;

    info!("LLM Response: {}", response.text);

    Ok(())
}
