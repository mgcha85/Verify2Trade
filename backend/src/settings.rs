use config::{Config, ConfigError, File};
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Deserialize, Clone)]
pub struct Backtest {
    pub data_path: String,
    pub default_symbol: String,
    pub initial_capital: f64,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Ai {
    pub prompts: HashMap<String, String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Settings {
    pub backtest: Backtest,
    pub ai: Ai,
}

impl Settings {
    pub fn new() -> Result<Self, ConfigError> {
        let builder = Config::builder();

        let config_path = if std::path::Path::new("config.yaml").exists() {
            "config.yaml"
        } else if std::path::Path::new("../config.yaml").exists() {
            "../config.yaml"
        } else {
            "config.yaml"
        };

        let s = builder.add_source(File::with_name(config_path)).build()?;

        s.try_deserialize()
    }
}
