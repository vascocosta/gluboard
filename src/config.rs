use std::path::PathBuf;

use anyhow::{Context, Result};
use serde::Deserialize;
use tokio::fs::read_to_string;

#[derive(Deserialize)]
pub struct Config {
    pub banner_file: Option<PathBuf>,
    pub hostname: String,
    pub port: u16,
    pub welcome_msg: Option<String>,
}

impl Config {
    pub async fn from_file() -> Result<Self> {
        match read_to_string("config.toml").await {
            Ok(toml) => Ok(toml::from_str(&toml).context("Could not parse config.toml")?),
            Err(_) => match read_to_string("config.json").await {
                Ok(json) => Ok(serde_json::from_str(&json).context("Could not parse config.json")?),
                Err(e) => {
                    eprintln!("{e}: Could not access any configuration files, using defaults");
                    Ok(Self::default())
                }
            },
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            banner_file: None,
            hostname: "127.0.0.1".to_string(),
            port: 1981,
            welcome_msg: Some("Welcome to this BBS!".to_string()),
        }
    }
}
