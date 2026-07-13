use std::process::Command;

use anyhow::{Context, Result, bail};
use serde::Deserialize;

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

// Shapes matching `aws ec2 describe-instances --output json`, only
// capturing the fields this tool actually needs.
#[derive(Deserialize)]
struct DescribeInstancesOutput {
    #[serde(rename = "Reservations")]
    reservations: Vec<Reservation>,
}

#[derive(Deserialize)]
struct Reservation {
    #[serde(rename = "Instances")]
    instances: Vec<Instance>,
}

#[derive(Deserialize)]
struct Instance {
    #[serde(rename = "InstanceId")]
    instance_id: String,
    #[serde(rename = "State")]
    state: InstanceState,
    #[serde(rename = "Tags", default)]
    tags: Vec<Tag>,
}

#[derive(Deserialize)]
struct InstanceState {
    #[serde(rename = "Name")]
    name: String,
}

#[derive(Deserialize)]
struct Tag {
    #[serde(rename = "Key")]
    key: String,
    #[serde(rename = "Value")]
    value: String,
}

// Shapes matching `aws ssm send-command --output json` /
// `aws ssm get-command-invocation --output json`, only capturing the
// fields this tool actually needs.
#[derive(Deserialize)]
struct SendCommandOutput {
    #[serde(rename = "Command")]
    command: SendCommandCommand,
}

#[derive(Deserialize)]
struct SendCommandCommand {
    #[serde(rename = "CommandId")]
    command_id: String,
}

#[derive(Deserialize)]
struct CommandInvocation {
    #[serde(rename = "Status")]
    status: String,
    #[serde(rename = "StandardOutputContent", default)]
    standard_output_content: String,
    #[serde(rename = "StandardErrorContent", default)]
    standard_error_content: String,
}

/// Thin wrapper around the `aws` CLI, scoped to the EC2 operations this
/// tool needs (list, start/wait, stop/wait). Shells out to the CLI rather
/// than using the AWS SDK so it can rely on whatever `aws` install/config
/// (profiles, SSO, etc.) the user already has, and to keep this binary
/// small.
pub struct Ec2Client {
    profile: String,
}

impl Ec2Client {
    pub fn new(profile: &str) -> Self {
        Self {
            profile: profile.to_string(),
        }
    }

    /// Runs `aws <args...> --profile <profile>`, returning the raw output.
    /// Fails with a helpful message if the `aws` executable can't be
    /// found/launched at all.
    fn run_aws(&self, args: &[&str]) -> Result<std::process::Output> {
        Command::new("aws")
            .args(args)
            .arg("--profile")
            .arg(&self.profile)
            .output()
            .context(
                "failed to run the `aws` CLI - is it installed and on PATH? \
                 (see https://aws.amazon.com/cli/)",
            )
    }

    /// Runs `aws <args...> --profile <profile> --output json` and parses
    /// the stdout as JSON. Fails with the CLI's stderr message if it
    /// exited unsuccessfully.
    fn run_aws_json<T: for<'de> Deserialize<'de>>(&self, args: &[&str]) -> Result<T> {
        let mut full_args = args.to_vec();
        full_args.push("--output");
        full_args.push("json");

        let output = self.run_aws(&full_args)?;
        if !output.status.success() {
            bail!(
                "aws {} failed: {}",
                args.join(" "),
                String::from_utf8_lossy(&output.stderr).trim()
            );
        }

        serde_json::from_slice(&output.stdout).context("failed to parse `aws` CLI JSON output")
    }

    /// Runs `describe-instances` with the given extra filter args (e.g.
    /// `--instance-ids <id>`), mapping the result down to `InstanceEntry`.
    fn describe_instances(&self, extra_args: &[&str]) -> Result<Vec<InstanceEntry>> {
        let mut args = vec!["ec2", "describe-instances"];
        args.extend_from_slice(extra_args);

        let out: DescribeInstancesOutput = self
            .run_aws_json(&args)
            .context("failed to describe instances")?;

        let entries = out
            .reservations
            .into_iter()
            .flat_map(|r| r.instances)
            .map(|i| {
                let name = i
                    .tags
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
        self.describe_instances(&[])
    }

    /// Fetches the current state (e.g. "running", "stopped") of a single
    /// instance, or `None` if it doesn't exist.
    pub fn instance_state(&self, instance_id: &str) -> Result<Option<String>> {
        let entries = self
            .describe_instances(&["--instance-ids", instance_id])
            .with_context(|| format!("failed to describe instance {instance_id}"))?;

        Ok(entries.into_iter().next().map(|e| e.state))
    }

    /// Starts the given instance and waits until it reaches the `running`
    /// state.
    pub fn start_and_wait(&self, instance_id: &str) -> Result<()> {
        println!("Starting instance {instance_id}...");
        let output = self.run_aws(&[
            "ec2",
            "start-instances",
            "--instance-ids",
            instance_id,
            "--no-cli-pager",
        ])?;
        if !output.status.success() {
            bail!(
                "failed to start instance {instance_id}: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            );
        }

        println!("Waiting for instance {instance_id} to reach 'running'...");
        let wait_output = self.run_aws(&[
            "ec2",
            "wait",
            "instance-running",
            "--instance-ids",
            instance_id,
        ])?;
        if !wait_output.status.success() {
            bail!(
                "failed while waiting for instance {instance_id} to become running: {}",
                String::from_utf8_lossy(&wait_output.stderr).trim()
            );
        }

        println!("Instance {instance_id} is now running.");
        Ok(())
    }

    /// Stops the given instance and waits until it reaches the `stopped`
    /// state.
    pub fn stop_and_wait(&self, instance_id: &str) -> Result<()> {
        println!("Stopping instance {instance_id}...");
        let output = self.run_aws(&[
            "ec2",
            "stop-instances",
            "--instance-ids",
            instance_id,
            "--no-cli-pager",
        ])?;
        if !output.status.success() {
            bail!(
                "failed to stop instance {instance_id}: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            );
        }

        println!("Waiting for instance {instance_id} to reach 'stopped'...");
        let wait_output = self.run_aws(&[
            "ec2",
            "wait",
            "instance-stopped",
            "--instance-ids",
            instance_id,
        ])?;
        if !wait_output.status.success() {
            bail!(
                "failed while waiting for instance {instance_id} to become stopped: {}",
                String::from_utf8_lossy(&wait_output.stderr).trim()
            );
        }

        println!("Instance {instance_id} is now stopped.");
        Ok(())
    }

    /// Schedules an OS-level shutdown inside the instance in `minutes`
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
    pub fn schedule_shutdown(
        &self,
        instance_id: &str,
        minutes: i64,
        target_time: &str,
    ) -> Result<()> {
        let script = format!(
            "if shutdown --show >/dev/null 2>&1; then \
                echo 'Shutdown already scheduled, leaving it as-is:'; \
                shutdown --show 2>&1; \
             else \
                shutdown -h +{minutes} 'Auto-shutdown scheduled by aws-util' && \
                echo 'Scheduled shutdown at {target_time} (in {minutes} minute(s)).'; \
             fi"
        );

        println!("Sending SSM command to schedule shutdown on {instance_id}...");
        let params = format!("commands=[\"{script}\"]");
        let send_output: SendCommandOutput = self.run_aws_json(&[
            "ssm",
            "send-command",
            "--instance-ids",
            instance_id,
            "--document-name",
            "AWS-RunShellScript",
            "--parameters",
            &params,
        ])?;
        let command_id = send_output.command.command_id;

        // Use the CLI's built-in waiter (handles the "invocation not
        // registered yet" race and polling internally) instead of a
        // manual poll loop. It errors out if the command reaches a
        // failure/cancellation status.
        let wait_output = self.run_aws(&[
            "ssm",
            "wait",
            "command-executed",
            "--command-id",
            &command_id,
            "--instance-id",
            instance_id,
        ])?;

        let invocation: CommandInvocation = self.run_aws_json(&[
            "ssm",
            "get-command-invocation",
            "--command-id",
            &command_id,
            "--instance-id",
            instance_id,
        ])?;

        if !invocation.standard_output_content.trim().is_empty() {
            println!("{}", invocation.standard_output_content.trim());
        }

        if !wait_output.status.success() {
            bail!(
                "SSM command to schedule shutdown on {instance_id} did not succeed \
                 (status: {}): {}",
                invocation.status,
                invocation.standard_error_content.trim()
            );
        }

        Ok(())
    }
}
