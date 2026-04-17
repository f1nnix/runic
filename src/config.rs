use anyhow::{Context, Result};
use serde::Deserialize;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub ssh: SshConfig,
    #[serde(default)]
    pub picker: PickerConfig,
}

#[derive(Debug, Deserialize)]
pub struct SshConfig {
    #[serde(default = "default_exclude")]
    pub exclude: Vec<String>,
    #[serde(default)]
    pub include: Vec<String>,
}

impl Default for SshConfig {
    fn default() -> Self {
        Self {
            exclude: default_exclude(),
            include: Vec::new(),
        }
    }
}

fn default_exclude() -> Vec<String> {
    vec![
        "github.com".into(),
        "git.*".into(),
        "gitlab.*".into(),
        "bitbucket.*".into(),
    ]
}

#[derive(Debug, Deserialize)]
pub struct PickerConfig {
    #[serde(default = "default_height")]
    pub height: String,
}

impl Default for PickerConfig {
    fn default() -> Self {
        Self {
            height: default_height(),
        }
    }
}

fn default_height() -> String {
    "50%".into()
}

pub fn config_path() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("runic").join("config.toml"))
}

pub fn load() -> Result<Config> {
    let Some(path) = config_path() else {
        return Ok(Config::default());
    };
    if !path.is_file() {
        return Ok(Config::default());
    }
    let text = fs::read_to_string(&path)
        .with_context(|| format!("read {}", path.display()))?;
    toml::from_str(&text).with_context(|| format!("parse {}", path.display()))
}
