use std::process::{Command, Output};

use anyhow::{Context, Result, bail};
use serde::de::DeserializeOwned;

pub struct AwsCommand {
    cmd: Command,
}

impl AwsCommand {
    fn new() -> Self {
        Self {
            cmd: Command::new("aws"),
        }
    }

    fn add_profile(mut self, profile: &str) -> Self {
        self.cmd.arg("--profile").arg(profile);
        self
    }

    fn arg(mut self, arg: &str) -> Self {
        self.cmd.arg(arg);
        self
    }

    /// Renders the command roughly as it'd be typed on a shell, for use
    /// in error messages.
    fn command_line(&self) -> String {
        let program = self.cmd.get_program().to_string_lossy();
        let args = self
            .cmd
            .get_args()
            .map(|a| a.to_string_lossy())
            .collect::<Vec<_>>()
            .join(" ");
        format!("{program} {args}")
    }

    /// Runs the underlying command, failing with a helpful message
    /// (including the command line) if the `aws` executable can't be
    /// found/launched at all, or if it runs but exits unsuccessfully
    /// (including its stderr).
    fn run(&mut self) -> Result<Output> {
        let output = self
            .cmd
            .output()
            .with_context(|| format!("failed to run `{}`", self.command_line()))?;

        if !output.status.success() {
            bail!(
                "`{}` failed: {}",
                self.command_line(),
                String::from_utf8_lossy(&output.stderr).trim()
            );
        }

        Ok(output)
    }

    /// Runs the command and returns stdout as a UTF-8 string.
    fn output_text(mut self) -> Result<String> {
        let output = self.run()?;
        String::from_utf8(output.stdout)
            .with_context(|| format!("`{}` returned non-UTF-8 output", self.command_line()))
    }

    /// Adds `--output json`, runs the command, and parses stdout as JSON.
    fn output_json<T: DeserializeOwned>(&mut self) -> Result<T> {
        self.cmd.arg("--output").arg("json");
        let output = self.run()?;
        serde_json::from_slice(&output.stdout)
            .with_context(|| format!("failed to parse JSON output of `{}`", self.command_line()))
    }

    pub fn describe_instances<T: DeserializeOwned>(
        profile: &str,
        with_id: Option<&str>,
    ) -> Result<T> {
        let mut cmd = Self::new().arg("ec2").arg("describe-instances");

        if let Some(id) = with_id {
            cmd = cmd.arg("--instance-ids").arg(id);
        }

        cmd.add_profile(profile).output_json()
    }

    pub fn list_profiles() -> Result<String> {
        Self::new()
            .arg("configure")
            .arg("list-profiles")
            .output_text()
    }

    pub fn start_instances(profile: &str, instance_id: &str) -> Result<String> {
        Self::new()
            .arg("ec2")
            .arg("start-instances")
            .arg("--instance-ids")
            .arg(instance_id)
            .arg("--no-cli-pager")
            .add_profile(profile)
            .output_text()
    }

    pub fn wait_instance_running(profile: &str, instance_id: &str) -> Result<String> {
        Self::new()
            .arg("ec2")
            .arg("wait")
            .arg("instance-running")
            .arg("--instance-ids")
            .arg(instance_id)
            .add_profile(profile)
            .output_text()
    }

    pub fn stop_instances(profile: &str, instance_id: &str) -> Result<String> {
        Self::new()
            .arg("ec2")
            .arg("stop-instances")
            .arg("--instance-ids")
            .arg(instance_id)
            .arg("--no-cli-pager")
            .add_profile(profile)
            .output_text()
    }

    pub fn wait_instance_stopped(profile: &str, instance_id: &str) -> Result<String> {
        Self::new()
            .arg("ec2")
            .arg("wait")
            .arg("instance-stopped")
            .arg("--instance-ids")
            .arg(instance_id)
            .add_profile(profile)
            .output_text()
    }

    pub fn send_command<T: DeserializeOwned>(
        profile: &str,
        instance_id: &str,
        document_name: &str,
        parameters: &str,
    ) -> Result<T> {
        Self::new()
            .arg("ssm")
            .arg("send-command")
            .arg("--instance-ids")
            .arg(instance_id)
            .arg("--document-name")
            .arg(document_name)
            .arg("--parameters")
            .arg(parameters)
            .add_profile(profile)
            .output_json()
    }

    pub fn wait_command_executed(
        profile: &str,
        command_id: &str,
        instance_id: &str,
    ) -> Result<String> {
        Self::new()
            .arg("ssm")
            .arg("wait")
            .arg("command-executed")
            .arg("--command-id")
            .arg(command_id)
            .arg("--instance-id")
            .arg(instance_id)
            .add_profile(profile)
            .output_text()
    }

    pub fn get_command_invocation<T: DeserializeOwned>(
        profile: &str,
        command_id: &str,
        instance_id: &str,
    ) -> Result<T> {
        Self::new()
            .arg("ssm")
            .arg("get-command-invocation")
            .arg("--command-id")
            .arg(command_id)
            .arg("--instance-id")
            .arg(instance_id)
            .add_profile(profile)
            .output_json()
    }
}
