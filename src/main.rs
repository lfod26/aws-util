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

/// Runs the interactive configuration procedure: prompts for a profile
/// name, lists that profile's EC2 instances for fuzzy-selection, saves the
/// result to the config file, and returns it.
async fn run_configure() -> Result<AwsListConfig> {
    let profile = interactive::prompt_profile()?;

    let client = Ec2Client::new(&profile).await;
    let entries = client.list_instances().await?;
    if entries.is_empty() {
        bail!("No instances found");
    }

    let selected = interactive::select_instance(&entries)?;
    println!("Selected instance: {selected}");

    let config = AwsListConfig::new(profile, selected.instance_id.clone());
    config.save()?;
    Ok(config)
}

#[tokio::main]
async fn main() -> Result<()> {
    signal::install_ctrlc_handler()?;

    let cli = Cli::parse();

    let config = if cli.configure {
        run_configure().await?
    } else {
        match AwsListConfig::load()? {
            Some(config) => config,
            None => {
                println!("No config found, let's set one up.");
                run_configure().await?
            }
        }
    };

    let client = Ec2Client::new(&config.profile).await;

    match client.instance_state(&config.instance_id).await? {
        Some(state) if state == "running" => {
            println!("Instance {} is already running.", config.instance_id);
        }
        _ => {
            client.start_and_wait(&config.instance_id).await?;
        }
    }

    Ok(())
}
