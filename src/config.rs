use std::path::PathBuf;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

const CONFIG_NAME: &str = "aws_list_conf.json";

#[derive(Serialize, Deserialize)]
pub struct AwsListConfig {
    pub profile: String,
    pub instance_id: String,
}

impl AwsListConfig {
    pub fn new(profile: String, instance_id: String) -> Self {
        Self {
            profile,
            instance_id,
        }
    }

    /// Resolves the path to the config file next to the running executable
    /// (or the current directory in debug builds, for convenience during
    /// `cargo run`), mirroring the convention used by the sibling `aws-util`
    /// tool.
    fn resolve_path() -> std::io::Result<PathBuf> {
        #[cfg(debug_assertions)]
        let mut base = std::env::current_dir()?;

        #[cfg(not(debug_assertions))]
        let mut base = std::env::current_exe().map(|mut p| {
            p.pop();
            p
        })?;

        base.push(CONFIG_NAME);
        Ok(base)
    }

    /// Loads the config file next to the executable, if it exists.
    /// Returns `Ok(None)` when there is no config file yet.
    pub fn load() -> Result<Option<Self>> {
        let path = Self::resolve_path().context("failed to resolve config file path")?;

        if !path.exists() {
            return Ok(None);
        }

        let file = std::fs::File::open(&path)
            .with_context(|| format!("failed to open config file at {}", path.display()))?;
        let config = serde_json::from_reader(file)
            .with_context(|| format!("failed to parse config file at {}", path.display()))?;

        Ok(Some(config))
    }

    /// Saves this config to the config file next to the executable.
    pub fn save(&self) -> Result<()> {
        let path = Self::resolve_path().context("failed to resolve config file path")?;
        let file = std::fs::File::create(&path)
            .with_context(|| format!("failed to create config file at {}", path.display()))?;
        serde_json::to_writer_pretty(file, self)
            .with_context(|| format!("failed to write config file at {}", path.display()))?;

        println!(
            "Saved profile '{}' and instance id '{}' to {}",
            self.profile,
            self.instance_id,
            path.display()
        );
        Ok(())
    }
}
