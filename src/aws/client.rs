//! Thin wrapper around the `aws` CLI: a profile-scoped client for shelling
//! out to `aws` subcommands and parsing their JSON output.

use anyhow::{Context, Result, bail};
use serde::Deserialize;

use super::aws_command::AwsCommand;
use super::instance::{Instance, InstanceEntry};

// Shapes matching `aws ec2 describe-instances --output json`, only
// capturing the fields this tool actually needs.
#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct DescribeInstancesOutput {
    reservations: Vec<Reservation>,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Reservation {
    instances: Vec<InstanceJson>,
}

// Named `InstanceJson` (rather than `Instance`) to avoid colliding with
// the public `Instance` handle type in `instance.rs`, which represents
// something different (a single instance scoped to a client, for
// start/stop/etc.).
#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct InstanceJson {
    instance_id: String,
    state: InstanceState,
    tags: Option<Vec<Tag>>,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct InstanceState {
    name: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Tag {
    key: String,
    value: String,
}

/// Lists the AWS CLI profile names configured on this machine, via
/// `aws configure list-profiles`. This isn't scoped to `AwsClient` since
/// it doesn't take a `--profile` argument itself (it's how a profile gets
/// picked in the first place).
///
/// Fails with a helpful message if the `aws` executable can't be
/// found/launched at all, if the command itself fails, or if it succeeds
/// but returns no profiles (meaning the user hasn't run `aws configure`
/// yet).
pub fn list_profiles() -> Result<Vec<String>> {
    let stdout = AwsCommand::list_profiles()?;

    let profiles: Vec<String> = stdout
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(String::from)
        .collect();

    if profiles.is_empty() {
        bail!(
            "No AWS CLI profiles found. Run `aws configure` (or `aws configure sso`) \
             to set one up first."
        );
    }

    Ok(profiles)
}

/// Thin wrapper around the `aws` CLI, scoped to the EC2 operations this
/// tool needs (list, start/wait, stop/wait). Shells out to the CLI rather
/// than using the AWS SDK so it can rely on whatever `aws` install/config
/// (profiles, SSO, etc.) the user already has, and to keep this binary
/// small.
pub struct AwsClient {
    profile: String,
}

impl AwsClient {
    pub fn new(profile: &str) -> Self {
        Self {
            profile: profile.to_string(),
        }
    }

    pub(super) fn profile(&self) -> &str {
        &self.profile
    }

    /// Runs `describe-instances`, optionally scoped to a single instance
    /// ID, mapping the result down to `InstanceEntry`.
    pub(super) fn describe_instances(&self, with_id: Option<&str>) -> Result<Vec<InstanceEntry>> {
        let out: DescribeInstancesOutput = AwsCommand::describe_instances(&self.profile, with_id)
            .context("failed to describe instances")?;

        let entries = out
            .reservations
            .into_iter()
            .flat_map(|r| r.instances)
            .map(|i| {
                let name = i
                    .tags
                    .as_deref()
                    .unwrap_or_default()
                    .iter()
                    .find(|t| t.key == "Name")
                    .map(|t| t.value.clone())
                    .unwrap_or_else(|| "(no Name tag)".to_string());

                InstanceEntry {
                    instance_id: i.instance_id,
                    name,
                    state: i.state.name,
                }
            })
            .collect();

        Ok(entries)
    }

    /// Fetches all EC2 instances visible to the configured profile.
    /// Does not perform any selection, printing, or side effects.
    pub fn list_instances(&self) -> Result<Vec<InstanceEntry>> {
        self.describe_instances(None)
    }

    /// Returns a handle scoped to a single instance, so operations on it
    /// (state checks, start/stop, schedule-shutdown) don't need to keep
    /// passing the instance ID around.
    pub fn instance(&self, instance_id: &str) -> Instance<'_> {
        Instance::new(self, instance_id)
    }
}
