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
    mut messages: Vec<Message>,
    tools: Vec<Box<dyn Tool>>,
    print_usage: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let llm =
        llm::LlmClient::new(config.model_name, config.api_key, config.api_url);

    let tool_definitions =
        tools.iter().map(|t| t.definition().clone()).collect::<Vec<_>>();

    let mut out = io::BufWriter::new(io::stdout().lock());
    loop {
        let tool_definitions = tool_definitions.clone();
        let response = llm.get_chat_completion_streaming(
            &LlmRequest::new(messages.clone(), tool_definitions.clone()),
            |chunk| {
                write!(out, "{chunk}").unwrap();
                out.flush().unwrap();
            },
        )?;

        writeln!(out)?;
        out.flush()?;

        if print_usage {
            writeln!(out, "{:?}", response.usage())?;
        }

        out.flush()?;

        if response.tool_calls().is_empty() {
            break;
        }

        // Add the response message to the messages:
        {
            let assistant_message = Message::Assistant(
                response.text().to_owned(),
                match response.tool_calls() {
                    &[] => None,
                    tool_calls => Some(tool_calls.to_vec()),
                },
            );
            messages.push(assistant_message);
        }

        info!("{:?}", response.tool_calls());

        // Invoke the tool:
        let tool_message = {
            let first_tool = response.tool_calls().first().unwrap();
            let matching_tool = tools
                .iter()
                .find(|t| t.definition().name() == first_tool.name())
                .ok_or_else(|| {
                    format!("Tool {} not found", first_tool.name())
                })?;

            let tool_output =
                invoke_tool(matching_tool.as_ref(), first_tool.arguments())?;

            Message::Tool {
                content: tool_output,
                id: first_tool.id().to_owned(),
            }
        };

        // add a tool message
        messages.push(tool_message);
    }

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

fn invoke_tool(
    tool: &dyn Tool,
    args: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    info!("Invoking tool: {}", tool.definition().name());

    let args_parsed = serde_json::from_str(args)?;

    let output = tool.execute(args_parsed);

    info!("Tool output: {output:?}");

    output
}
