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

/// Config file contents: a list of profile+instance groups. Supports
/// multiple groups so this tool can manage more than one instance (each
/// possibly under a different AWS CLI profile).
#[derive(Serialize, Deserialize, Default)]
pub struct AwsUtilConfig {
    #[serde(default)]
    pub groups: Vec<ProfileGroup>,

    // Older config files (before multi-profile support) stored a single
    // profile/instance_id pair at the top level instead of in `groups`.
    // These fields are only read during `load()` to migrate such files
    // into a single-entry `groups` list, and are never written back out.
    #[serde(default, skip_serializing)]
    profile: Option<String>,
    #[serde(default, skip_serializing)]
    instance_id: Option<String>,
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
    /// returned. Old-style single profile/instance_id configs (from
    /// before multi-profile support) are migrated into a one-entry
    /// `groups` list.
    pub fn load() -> Result<Self> {
        let path = Self::resolve_path().context("failed to resolve config file path")?;

        if !path.exists() {
            return Ok(Self::default());
        }

        let file = std::fs::File::open(&path)
            .with_context(|| format!("failed to open config file at {}", path.display()))?;

        match serde_json::from_reader::<_, Self>(file) {
            Ok(mut config) => {
                if config.groups.is_empty()
                    && let (Some(profile), Some(instance_id)) =
                        (config.profile.take(), config.instance_id.take())
                {
                    config.groups.push(ProfileGroup {
                        profile,
                        instance_id,
                    });
                }
                Ok(config)
            }
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
