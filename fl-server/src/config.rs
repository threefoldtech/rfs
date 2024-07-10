use crate::handler::FlistState;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fs, sync::Mutex};

#[derive(Debug, Clone, Serialize, Eq, Hash, PartialEq)]
pub struct JobID(pub String);

// add configs
#[derive(Debug)]
pub struct AppState {
    pub jobs_state: Mutex<HashMap<JobID, FlistState>>,
}

#[derive(Debug, Default, Clone, Deserialize)]
pub struct Config {
    pub host: String,
    pub port: i16,
    pub store_url: Vec<String>,
    pub flist_dir: String,
    pub version: String,

    // TODO: add token for every username
    pub tokens_file_path: String,
}

// TODO: validate
/// Parse the config file into Config struct.
pub fn parse_config(filepath: &str) -> Result<Config> {
    let content = fs::read_to_string(filepath).context("failed to read config file")?;
    let c: Config = toml::from_str(&content).context("failed to convert toml config data")?;
    fs::create_dir_all(&c.flist_dir)?;
    Ok(c)
}
