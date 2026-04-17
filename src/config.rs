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
    #[serde(default)]
    pub shell: ShellConfig,
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

#[derive(Debug, Deserialize)]
pub struct ShellConfig {
    /// How long the shell waits for a follow-up byte after Esc, in milliseconds.
    ///
    /// Default: 10 (near-instant). Zsh's `KEYTIMEOUT` uses 10ms units, so this is
    /// divided by 10 for zsh. Bash's `keyseq-timeout` uses ms directly.
    /// Set to 0 to leave the shell's existing timeout untouched.
    #[serde(default = "default_key_timeout_ms")]
    pub key_timeout_ms: u32,
}

impl Default for ShellConfig {
    fn default() -> Self {
        Self {
            key_timeout_ms: default_key_timeout_ms(),
        }
    }
}

fn default_key_timeout_ms() -> u32 {
    10
}

pub fn config_path() -> Option<PathBuf> {
    if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
        if !xdg.is_empty() {
            return Some(PathBuf::from(xdg).join("runic").join("config.toml"));
        }
    }
    dirs::home_dir().map(|h| h.join(".config").join("runic").join("config.toml"))
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
