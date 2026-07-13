use clap::Parser;

/// Starts the configured EC2 instance (waiting for it to reach the
/// `running` state), reading the profile and instance id from a config
/// file next to the executable. If no config exists yet, prints a message
/// telling the user to run `--configure` first (no interactive prompting
/// happens outside of `--configure`).
#[derive(Parser)]
#[command(
    name = "aws-util",
    about = "Start or stop a configured EC2 instance via the `aws` CLI"
)]
pub struct Cli {
    /// Run the interactive configuration procedure (choose profile and
    /// instance), always prompting for both even if a config already
    /// exists, to change it. Only configures — does not start or stop the
    /// instance.
    #[arg(long, conflicts_with_all = ["stop", "schedule_shutdown"])]
    pub configure: bool,

    /// Start the configured EC2 instance (waiting for it to reach the
    /// `running` state). This is the default behavior when no flag is
    /// passed; the flag exists to make the action explicit.
    #[arg(long, conflicts_with_all = ["stop", "schedule_shutdown"])]
    pub start: bool,

    /// Stop the configured EC2 instance (waiting for it to reach the
    /// `stopped` state) instead of starting it.
    #[arg(long, conflicts_with_all = ["start", "schedule_shutdown"])]
    pub stop: bool,

    /// Schedule an OS-level shutdown inside the instance at the given
    /// local clock time (24-hour HH:MM, e.g. `18:30`; default `18:30` if
    /// no value is given). Rolls over to tomorrow if that time has
    /// already passed today. Delivered via SSM Run Command. Instead of
    /// stopping the instance from here, this tells the instance itself
    /// to shut down later — so it stops even if this tool isn't running
    /// to see it happen. Checks first whether a shutdown is already
    /// scheduled and leaves it alone if so.
    #[arg(
        long,
        value_name = "HH:MM",
        num_args = 0..=1,
        default_missing_value = "18:30",
        conflicts_with_all = ["configure", "start", "stop"]
    )]
    pub schedule_shutdown: Option<String>,
}
