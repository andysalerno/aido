use log::info;

use crate::{
    config::Config,
    llm::{self, LlmRequest},
};

pub(crate) fn run(config: Config, input: &str) -> Result<(), Box<dyn std::error::Error>> {
    let llm = llm::LlmClient::new(config.model_name, config.api_key, config.api_url);

    // let response = llm.get_chat_completion(&LlmRequest { text: input.into() })?;

    let response =
        llm.get_chat_completion_streaming(&LlmRequest { text: input.into() }, |chunk| {
            info!("{chunk}");
        })?;

    info!("LLM Response: {}", response.text());
    info!("{:?}", response.usage());

    Ok(())
}
