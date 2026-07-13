use std::path::PathBuf;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

const CONFIG_NAME: &str = "aws_util_conf.json";

/// Config file contents. Fields are optional so a partially-filled config
/// (e.g. one missing `instance_id`, or from an older version of this tool)
/// doesn't need to be discarded entirely — only the missing pieces need to
/// be prompted for again.
#[derive(Serialize, Deserialize, Default)]
pub struct AwsListConfig {
    pub profile: Option<String>,
    pub instance_id: Option<String>,
}

impl AwsListConfig {
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

    /// Loads the config file next to the executable. Returns a default
    /// (empty) config when there is no config file yet. Since all fields
    /// are optional, a config file that's just missing a field parses
    /// fine — only genuinely malformed JSON hits the error path, in which
    /// case the invalid file is backed up (so it isn't silently lost) and
    /// a warning is printed, falling back to an empty config.
    pub fn load() -> Result<Self> {
        let path = Self::resolve_path().context("failed to resolve config file path")?;

        if !path.exists() {
            return Ok(Self::default());
        }

        let file = std::fs::File::open(&path)
            .with_context(|| format!("failed to open config file at {}", path.display()))?;

        match serde_json::from_reader(file) {
            Ok(config) => Ok(config),
            Err(err) => {
                let backup_path = path.with_extension("json.bak");
                match std::fs::rename(&path, &backup_path) {
                    Ok(()) => eprintln!(
                        "Warning: config file at {} is invalid ({err}). \
                         Backed it up to {} and will reconfigure.",
                        path.display(),
                        backup_path.display()
                    ),
                    Err(rename_err) => eprintln!(
                        "Warning: config file at {} is invalid ({err}), and it could not be \
                         backed up ({rename_err}). It will be overwritten when reconfiguring.",
                        path.display()
                    ),
                }
                Ok(Self::default())
            }
        }
    }

    /// Saves this config to the config file next to the executable.
    pub fn save(&self) -> Result<()> {
        let path = Self::resolve_path().context("failed to resolve config file path")?;
        let file = std::fs::File::create(&path)
            .with_context(|| format!("failed to create config file at {}", path.display()))?;
        serde_json::to_writer_pretty(file, self)
            .with_context(|| format!("failed to write config file at {}", path.display()))?;

        println!("Saved config to {}", path.display());
        Ok(())
    }
}
