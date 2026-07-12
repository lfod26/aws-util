use clap::Parser;

/// Starts the configured EC2 instance (waiting for it to reach the
/// `running` state), reading the profile and instance id from a config
/// file next to the executable. If no config exists yet, prints a message
/// telling the user to run `--configure` first (no interactive prompting
/// happens outside of `--configure`).
#[derive(Parser)]
#[command(
    name = "aws-list",
    about = "Start a configured EC2 instance via the AWS SDK for Rust"
)]
pub struct Cli {
    /// Run the interactive configuration procedure (choose profile and
    /// instance), always prompting for both even if a config already
    /// exists, to change it. Only configures — does not start or stop the
    /// instance.
    #[arg(long, conflicts_with = "stop")]
    pub configure: bool,

    /// Start the configured EC2 instance (waiting for it to reach the
    /// `running` state). This is the default behavior when no flag is
    /// passed; the flag exists to make the action explicit.
    #[arg(long, conflicts_with = "stop")]
    pub start: bool,

    /// Stop the configured EC2 instance (waiting for it to reach the
    /// `stopped` state) instead of starting it.
    #[arg(long, conflicts_with = "start")]
    pub stop: bool,
}