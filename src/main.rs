mod cli;
mod config;
mod ec2;
mod interactive;
mod signal;

use anyhow::{Result, bail};
use clap::Parser;

use cli::Cli;
use config::AwsListConfig;
use ec2::Ec2Client;

/// Runs the interactive configuration procedure, always prompting for both
/// the profile and the instance (even if a config already exists), then
/// saves it. Does not start or stop the instance.
fn run_configure() -> Result<()> {
    let profile = interactive::prompt_profile()?;
    let client = Ec2Client::new(&profile);

    let entries = client.list_instances()?;
    if entries.is_empty() {
        bail!("No instances found");
    }
    let selected = interactive::select_instance(&entries)?;
    println!("Selected instance: {selected}");

    let config = AwsListConfig {
        profile: Some(profile),
        instance_id: Some(selected.instance_id.clone()),
    };
    config.save()?;

    Ok(())
}

fn main() -> Result<()> {
    signal::install_ctrlc_handler()?;

    let cli = Cli::parse();

    if cli.configure {
        return run_configure();
    }

    let config = AwsListConfig::load()?;
    let (Some(profile), Some(instance_id)) = (config.profile, config.instance_id) else {
        println!("No configuration found. Run `aws-list --configure` first.");
        return Ok(());
    };

    let client = Ec2Client::new(&profile);

    if cli.stop {
        match client.instance_state(&instance_id)? {
            Some(state) if state == "stopped" => {
                println!("Instance {instance_id} is already stopped.");
            }
            _ => {
                client.stop_and_wait(&instance_id)?;
            }
        }
        return Ok(());
    }

    match client.instance_state(&instance_id)? {
        Some(state) if state == "running" => {
            println!("Instance {instance_id} is already running.");
        }
        _ => {
            client.start_and_wait(&instance_id)?;
        }
    }

    Ok(())
}
