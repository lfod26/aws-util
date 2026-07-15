//! A single EC2 instance, scoped to a profile via the `AwsClient` that
//! created it.

use anyhow::{Context, Result, bail};
use serde::Deserialize;

use super::aws_command::AwsCommand;
use super::client::AwsClient;

// Shapes matching `aws ssm send-command --output json` /
// `aws ssm get-command-invocation --output json`, only capturing the
// fields this tool actually needs.
#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct SendCommandOutput {
    command: SendCommandCommand,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct SendCommandCommand {
    command_id: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct CommandInvocation {
    status: String,
    standard_output_content: Option<String>,
    standard_error_content: Option<String>,
}

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

/// A single EC2 instance, scoped to a profile via the `AwsClient` that
/// created it. Owns the instance ID so callers don't need to pass it to
/// every method.
pub struct Instance<'a> {
    client: &'a AwsClient,
    instance_id: String,
}

impl<'a> Instance<'a> {
    pub(super) fn new(client: &'a AwsClient, instance_id: &str) -> Self {
        Self {
            client,
            instance_id: instance_id.to_string(),
        }
    }

    /// Fetches the current state (e.g. "running", "stopped") of this
    /// instance, or `None` if it doesn't exist.
    pub fn state(&self) -> Result<Option<String>> {
        let instance_id = &self.instance_id;
        let entries = self
            .client
            .describe_instances(Some(instance_id))
            .with_context(|| format!("failed to describe instance {instance_id}"))?;

        Ok(entries.into_iter().next().map(|e| e.state))
    }

    /// Starts this instance and waits until it reaches the `running`
    /// state.
    pub fn start_and_wait(&self) -> Result<()> {
        let instance_id = &self.instance_id;
        println!("Starting instance {instance_id}...");
        AwsCommand::start_instances(self.client.profile(), instance_id)?;

        println!("Waiting for instance {instance_id} to reach 'running'...");
        AwsCommand::wait_instance_running(self.client.profile(), instance_id)?;

        println!("Instance {instance_id} is now running.");
        Ok(())
    }

    /// Stops this instance and waits until it reaches the `stopped`
    /// state.
    pub fn stop_and_wait(&self) -> Result<()> {
        let instance_id = &self.instance_id;
        println!("Stopping instance {instance_id}...");
        AwsCommand::stop_instances(self.client.profile(), instance_id)?;

        println!("Waiting for instance {instance_id} to reach 'stopped'...");
        AwsCommand::wait_instance_stopped(self.client.profile(), instance_id)?;

        println!("Instance {instance_id} is now stopped.");
        Ok(())
    }

    /// Schedules an OS-level shutdown inside this instance in `minutes`
    /// minutes from now (`target_time` is only used for the printed
    /// message), via SSM Run Command (`AWS-RunShellScript`). First checks
    /// (in the same remote script) whether a shutdown is already pending
    /// via `shutdown --show`'s exit code (0 = a shutdown is scheduled,
    /// 1 = none is), and leaves it alone if so instead of scheduling a
    /// second one.
    ///
    /// Requires the instance to have the SSM Agent running and an
    /// instance profile with SSM permissions - if not, the send-command
    /// call itself will fail with a clear error from the CLI.
    pub fn schedule_shutdown(&self, minutes: i64, target_time: &str) -> Result<()> {
        let instance_id = &self.instance_id;
        let script = format!(
            "if show=$(shutdown --show 2>&1); then \
                echo 'Shutdown already scheduled, leaving it as-is:'; \
                echo \"$show\"; \
             else \
                shutdown -h +{minutes} 'Auto-shutdown scheduled by aws-util' && \
                echo 'Scheduled shutdown at {target_time} (in {minutes} minute(s)).'; \
             fi"
        );

        println!("Sending SSM command to schedule shutdown on {instance_id}...");
        let params = format!("commands=[\"{script}\"]");
        let profile = self.client.profile();
        let send_output: SendCommandOutput = AwsCommand::send_command(
            profile,
            instance_id,
            "AWS-RunShellScript",
            &params,
        )?;
        let command_id = send_output.command.command_id;

        // Use the CLI's built-in waiter (handles the "invocation not
        // registered yet" race and polling internally) instead of a
        // manual poll loop. It errors out if the command reaches a
        // failure/cancellation status - in that case we still want the
        // detailed invocation status/stderr below, so the error itself
        // is only used as a "did it succeed" signal here rather than
        // being propagated directly.
        let wait_result = AwsCommand::wait_command_executed(profile, &command_id, instance_id);

        let invocation: CommandInvocation =
            AwsCommand::get_command_invocation(profile, &command_id, instance_id)?;

        if let Some(stdout) = invocation.standard_output_content.as_deref() {
            let stdout = stdout.trim();
            if !stdout.is_empty() {
                println!("{stdout}");
            }
        }

        if wait_result.is_err() {
            bail!(
                "SSM command to schedule shutdown on {instance_id} did not succeed \
                 (status: {}): {}",
                invocation.status,
                invocation
                    .standard_error_content
                    .as_deref()
                    .unwrap_or_default()
                    .trim()
            );
        }

        Ok(())
    }
}
