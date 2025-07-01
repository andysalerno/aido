use std::{io::Write, path::Path, vec};

use log::info;

use crate::{
    config::Config,
    llm::{self, LlmRequest, Message},
    tools::Tool,
};
use std::io::{self};

pub fn run(
    config: Config,
    messages: Vec<Message>,
    tools: Vec<Box<dyn Tool>>,
    print_usage: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let llm =
        llm::LlmClient::new(config.model_name, config.api_key, config.api_url);

    let mut out = io::BufWriter::new(io::stdout().lock());

    let response = llm.get_chat_completion_streaming(
        &LlmRequest::new(messages, tools),
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

pub fn run_recipe(
    config: Config,
    recipes_dir: &Path,
    recipe_name: &str,
    user_message: Option<String>,
    tools: Vec<Box<dyn Tool>>,
    print_usage: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let recipe = crate::recipe::get(recipes_dir, recipe_name)?;

    info!("Running recipe: {}", recipe.header().name());

    let messages = {
        let mut messages = vec![Message::System(recipe.body().to_owned())];

        if let Some(user_msg) = user_message {
            messages.push(Message::User(user_msg));
        }

        messages
    };

    run(config, messages, tools, print_usage)
}
