use std::path::PathBuf;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

const CONFIG_NAME: &str = "aws_util_conf.json";

/// A single profile + instance pairing that `--start`/`--stop`/
/// `--schedule-shutdown` can act on.
#[derive(Serialize, Deserialize, Clone)]
pub struct ProfileGroup {
    pub profile: String,
    pub instance_id: String,
}

impl std::fmt::Display for ProfileGroup {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} ({})", self.profile, self.instance_id)
    }
}

/// Config file contents: a top-level JSON array of profile+instance
/// groups. Supports multiple groups so this tool can manage more than
/// one instance (each possibly under a different AWS CLI profile).
#[derive(Default)]
pub struct AwsUtilConfig {
    pub groups: Vec<ProfileGroup>,
}

impl AwsUtilConfig {
    /// Resolves the path to the config file next to the running executable
    /// (or the current directory in debug builds, for convenience during
    /// `cargo run`).
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
    /// (empty) config when there is no config file yet. If genuinely
    /// malformed JSON is found, the invalid file is backed up (so it
    /// isn't silently lost), a warning is printed, and an empty config is
    /// returned.
    pub fn load() -> Result<Self> {
        let path = Self::resolve_path().context("failed to resolve config file path")?;

        if !path.exists() {
            return Ok(Self::default());
        }

        let contents = std::fs::read_to_string(&path)
            .with_context(|| format!("failed to open config file at {}", path.display()))?;

        match serde_json::from_str::<Vec<ProfileGroup>>(&contents) {
            Ok(groups) => Ok(Self { groups }),
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
        serde_json::to_writer_pretty(file, &self.groups)
            .with_context(|| format!("failed to write config file at {}", path.display()))?;

        println!("Saved config to {}", path.display());
        Ok(())
    }
}
