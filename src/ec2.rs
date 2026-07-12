use std::time::Duration;

use anyhow::{Context, Result};
use aws_sdk_ec2::client::Waiters;

/// A single EC2 instance's relevant fields for display/selection.
pub struct InstanceEntry {
    pub instance_id: String,
    pub name: String,
    pub state: String,
}

impl std::fmt::Display for InstanceEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({}) [{}] {}", self.instance_id, self.state, self.name)
    }
}

/// Thin wrapper around the AWS SDK EC2 client, scoped to the operations
/// this tool needs (list, start, wait-until-running).
pub struct Ec2Client {
    client: aws_sdk_ec2::Client,
}

impl Ec2Client {
    pub async fn new(profile: &str) -> Self {
        let config = aws_config::defaults(aws_config::BehaviorVersion::latest())
            .profile_name(profile)
            .load()
            .await;
        Self {
            client: aws_sdk_ec2::Client::new(&config),
        }
    }

    /// Fetches all EC2 instances visible to the configured profile.
    /// Does not perform any selection, printing, or side effects.
    pub async fn list_instances(&self) -> Result<Vec<InstanceEntry>> {
        let resp = self
            .client
            .describe_instances()
            .send()
            .await
            .context("failed to describe instances")?;

        let entries = resp
            .reservations()
            .iter()
            .flat_map(|r| r.instances())
            .filter_map(|i| {
                let instance_id = i.instance_id()?.to_string();
                let name = i
                    .tags()
                    .iter()
                    .find(|t| t.key().is_some_and(|k| k == "Name"))
                    .and_then(|t| t.value())
                    .unwrap_or("(no Name tag)")
                    .to_string();
                let state = i
                    .state()
                    .and_then(|s| s.name())
                    .map(|n| n.as_str().to_string())
                    .unwrap_or_else(|| "unknown".to_string());

                Some(InstanceEntry {
                    instance_id,
                    name,
                    state,
                })
            })
            .collect();

        Ok(entries)
    }

    /// Fetches the current state (e.g. "running", "stopped") of a single
    /// instance, or `None` if it doesn't exist.
    pub async fn instance_state(&self, instance_id: &str) -> Result<Option<String>> {
        let resp = self
            .client
            .describe_instances()
            .instance_ids(instance_id)
            .send()
            .await
            .with_context(|| format!("failed to describe instance {instance_id}"))?;

        let state = resp
            .reservations()
            .iter()
            .flat_map(|r| r.instances())
            .find_map(|i| i.state().and_then(|s| s.name()).map(|n| n.as_str().to_string()));

        Ok(state)
    }

    /// Starts the given instance and waits until it reaches the `running`
    /// state.
    pub async fn start_and_wait(&self, instance_id: &str) -> Result<()> {
        println!("Starting instance {instance_id}...");
        self.client
            .start_instances()
            .instance_ids(instance_id)
            .send()
            .await
            .with_context(|| format!("failed to start instance {instance_id}"))?;

        println!("Waiting for instance {instance_id} to reach 'running'...");
        self.client
            .wait_until_instance_running()
            .instance_ids(instance_id)
            .wait(Duration::from_secs(90))
            .await
            .with_context(|| {
                format!("failed while waiting for instance {instance_id} to become running")
            })?;

        println!("Instance {instance_id} is now running.");
        Ok(())
    }
}
