use std::io::Write;

use crate::{
    config::Config,
    llm::{self, LlmRequest},
};
use std::io::{self};

pub fn run(
    config: Config,
    input: &str,
    print_usage: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let llm =
        llm::LlmClient::new(config.model_name, config.api_key, config.api_url);

    let mut out = io::BufWriter::new(io::stdout().lock());

    let response = llm.get_chat_completion_streaming(
        &LlmRequest { text: input.into() },
        |chunk| {
            write!(out, "{chunk}").unwrap();
            out.flush().unwrap();
        },
    )?;

    writeln!(out).unwrap();
    out.flush().unwrap();

    if print_usage {
        writeln!(out, "{:?}", response.usage()).unwrap();
    }

    out.flush().unwrap();

    Ok(())
}
