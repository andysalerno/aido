use std::path::Path;

use serde::{Deserialize, Serialize};

#[derive(Default, Debug, Serialize, Deserialize)]
pub struct Config {
    pub api_key: String,
    pub api_url: String,
    pub model_name: String,
    pub timeout: u64,
}

pub fn get_configuration_file_path() -> Result<String, Box<dyn std::error::Error>> {
    let path = confy::get_configuration_file_path("aido", None)?;
    Ok(path.to_string_lossy().to_string())
}

pub fn retrieve() -> Result<Config, Box<dyn std::error::Error>> {
    let cfg: Config = confy::load("aido", None)?;

    Ok(cfg)
}

pub fn retrieve_from_path(path: impl AsRef<Path>) -> Result<Config, Box<dyn std::error::Error>> {
    let cfg: Config = confy::load_path(path)?;

    Ok(cfg)
}
