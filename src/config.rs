use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

const DEFAULT_OUTPUT_DIR: &str = "Music/immersionpod/current";
const CONFIG_DIR: &str = "otopod";
const CONFIG_FILE: &str = "config.toml";

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    /// Output directory for condensed audio files.
    /// Supports ~ for home directory.
    pub output_dir: String,
}

impl Default for Config {
    fn default() -> Self {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("~"));
        Self {
            output_dir: home.join(DEFAULT_OUTPUT_DIR).to_string_lossy().to_string(),
        }
    }
}

impl Config {
    /// Returns the path to the config file: ~/.config/otopod/config.toml
    pub fn config_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("~/.config"))
            .join(CONFIG_DIR)
            .join(CONFIG_FILE)
    }

    /// Load config from disk. Creates a default config if it doesn't exist.
    pub fn load() -> Result<Self> {
        let path = Self::config_path();

        if !path.exists() {
            let config = Config::default();
            config.save()?;
            return Ok(config);
        }

        let content = std::fs::read_to_string(&path)
            .with_context(|| format!("Failed to read config: {}", path.display()))?;

        let config: Config = toml::from_str(&content)
            .with_context(|| format!("Failed to parse config: {}", path.display()))?;

        Ok(config)
    }

    /// Save config to disk.
    pub fn save(&self) -> Result<()> {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create config dir: {}", parent.display()))?;
        }

        let content = format!(
            "# otopod configuration\n# Edit output_dir to change where condensed audio files are saved.\n\n{}\n",
            toml::to_string(self).context("Failed to serialize config")?
        );

        std::fs::write(&path, content)
            .with_context(|| format!("Failed to write config: {}", path.display()))?;

        Ok(())
    }

    /// Resolve the output directory, expanding ~ to the home directory.
    pub fn resolved_output_dir(&self) -> PathBuf {
        if self.output_dir.starts_with('~') {
            let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("~"));
            home.join(self.output_dir.trim_start_matches("~/"))
        } else {
            PathBuf::from(&self.output_dir)
        }
    }
}
