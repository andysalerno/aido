use std::path::Path;

use serde::{Deserialize, Serialize};

#[derive(Default, Debug, Serialize, Deserialize)]
pub struct Config {
    pub api_key: String,
    pub api_url: String,
    pub timeout: u64,
}

pub fn retrieve() -> Result<Config, Box<dyn std::error::Error>> {
    let cfg: Config = confy::load("aido", None)?;

    Ok(cfg)
}

pub fn retrieve_from_path(path: impl AsRef<Path>) -> Result<Config, Box<dyn std::error::Error>> {
    let cfg: Config = confy::load_path(path)?;

    Ok(cfg)
}
