use serde::Deserialize;
use std::{collections::HashMap, env, fs};

use crate::errors::ScriptErrors;

#[derive(Deserialize, Debug)]
pub struct Config {
    pub branches: Branches,
    pub title: Title,
    pub template: Template,
    pub labels: Labels,
    pub commits: Commits,
    pub draft: bool,
    pub dry_run: bool,
}

#[derive(Deserialize, Debug)]
pub struct Branches {
    pub default: String,
    pub includes: HashMap<String, String>,
}

#[derive(Deserialize, Debug)]
pub struct Title {
    pub jira_prefixes: HashMap<String, String>,
    pub prefixes: HashMap<String, String>,
}

#[derive(Deserialize, Debug)]
pub struct Template {
    pub path: String,
}

#[derive(Deserialize, Debug)]
pub struct Labels {
    pub default: Vec<String>,
    pub includes: HashMap<String, Vec<String>>,
}

#[derive(Deserialize, Debug)]
pub struct Commits {
    pub prefixes: HashMap<String, String>,
}

pub fn get_config() -> Result<Config, ScriptErrors> {
    let exe_path = env::current_exe().map_err(|e| {
        ScriptErrors::ConfigError(format!("Failed to get current executable path: {}", e))
    })?;

    let exe_dir = exe_path.parent().ok_or(ScriptErrors::ConfigError(
        "Failed to get parent directory of executable".into(),
    ))?;
    let config_path = exe_dir.join("config.toml");

    let config = fs::read_to_string(config_path)
        .map_err(|e| ScriptErrors::ConfigError(format!("Failed to read config file: {}", e)))?;

    toml::from_str(&config).map_err(|e| ScriptErrors::ParseError(e.to_string()))
}
