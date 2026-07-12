use clap::Parser;

/// Starts the configured EC2 instance (waiting for it to reach the
/// `running` state), reading the profile and instance id from a config
/// file next to the executable. If no config exists yet, or `--configure`
/// is passed, runs an interactive configuration procedure first.
#[derive(Parser)]
#[command(
    name = "aws-list",
    about = "Start a configured EC2 instance via the AWS SDK for Rust"
)]
pub struct Cli {
    /// Run the interactive configuration procedure (choose profile and
    /// instance) even if a config already exists, to change it.
    #[arg(long)]
    pub configure: bool,
}
