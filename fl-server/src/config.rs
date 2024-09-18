use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs,
    path::PathBuf,
    sync::{Arc, Mutex},
};
use utoipa::ToSchema;

use crate::{
    db::{User, DB},
    handlers,
};

#[derive(Debug, ToSchema, Serialize, Clone)]
pub struct Job {
    pub id: String,
}

#[derive(ToSchema)]
pub struct AppState {
    pub jobs_state: Mutex<HashMap<String, handlers::FlistState>>,
    pub flists_progress: Mutex<HashMap<PathBuf, f32>>,
    pub db: Arc<dyn DB>,
    pub config: Config,
}

#[derive(Debug, Default, Clone, Deserialize)]
pub struct Config {
    pub host: String,
    pub port: usize,
    pub store_url: Vec<String>,
    pub flist_dir: String,

    pub jwt_secret: String,
    pub jwt_expire_hours: i64,
    pub users: Vec<User>,
}

/// Parse the config file into Config struct.
pub async fn parse_config(filepath: &str) -> Result<Config> {
    let content = fs::read_to_string(filepath).context("failed to read config file")?;
    let c: Config = toml::from_str(&content).context("failed to convert toml config data")?;

    if !hostname_validator::is_valid(&c.host) {
        return Err(anyhow::Error::msg(format!("host '{}' is invalid", c.host)));
    }

    if c.port > 65535 {
        return Err(anyhow::Error::msg(format!(
            "port '{}' is invalid, must be between [0, 65535]",
            c.port
        )));
    }

    rfs::store::parse_router(&c.store_url)
        .await
        .context("failed to parse store urls")?;
    fs::create_dir_all(&c.flist_dir).context("failed to create flists directory")?;

    if c.jwt_expire_hours < 1 || c.jwt_expire_hours > 24 {
        return Err(anyhow::Error::msg(format!(
            "jwt expiry interval in hours '{}' is invalid, must be between [1, 24]",
            c.jwt_expire_hours
        )));
    }

    Ok(c)
}
