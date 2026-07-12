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

/// Fills in any missing pieces of `config` by prompting the user, saving
/// the result back to the config file. If `--configure` was passed, the
/// caller instead passes a fully-empty config so everything is re-prompted.
async fn resolve_config(mut config: AwsListConfig) -> Result<(AwsListConfig, Ec2Client)> {
    let mut changed = false;

    if config.profile.is_none() {
        config.profile = Some(interactive::prompt_profile()?);
        changed = true;
    }
    let profile = config.profile.clone().expect("profile was just set");

    let client = Ec2Client::new(&profile).await;

    if config.instance_id.is_none() {
        let entries = client.list_instances().await?;
        if entries.is_empty() {
            bail!("No instances found");
        }

        let selected = interactive::select_instance(&entries)?;
        println!("Selected instance: {selected}");
        config.instance_id = Some(selected.instance_id.clone());
        changed = true;
    }

    if changed {
        config.save()?;
    }

    Ok((config, client))
}

#[tokio::main]
async fn main() -> Result<()> {
    signal::install_ctrlc_handler()?;

    let cli = Cli::parse();

    let loaded = if cli.configure {
        AwsListConfig::default()
    } else {
        AwsListConfig::load()?
    };

    let (config, client) = resolve_config(loaded).await?;
    let instance_id = config.instance_id.expect("instance_id was just resolved");

    match client.instance_state(&instance_id).await? {
        Some(state) if state == "running" => {
            println!("Instance {instance_id} is already running.");
        }
        _ => {
            client.start_and_wait(&instance_id).await?;
        }
    }

    Ok(())
}
