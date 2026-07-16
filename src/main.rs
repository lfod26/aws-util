mod cli;
mod config;
mod aws;
mod interactive;
mod schedule;
mod signal;
mod spinner;

use anyhow::{Result, bail};
use clap::Parser;
use console::style;

use cli::Cli;
use config::{AwsUtilConfig, ProfileGroup};
use aws::AwsClient;

/// Runs the interactive configuration procedure. If one or more groups
/// are already configured, first lets the user choose whether to edit one
/// of them (replacing it in place) or add a new one; if none are
/// configured yet, goes straight to adding the first one. Does not start
/// or stop any instance.
fn run_configure() -> Result<()> {
    let mut config = AwsUtilConfig::load()?;

    let edit_index = if config.groups.is_empty() {
        None
    } else {
        interactive::select_group_to_edit(&config.groups)?
    };

    let profiles = aws::list_profiles()?;
    let profile = interactive::select_profile(&profiles)?;
    let client = AwsClient::new(profile);

    let entries = spinner::with_spinner("Fetching instances...", || client.list_instances())?;
    if entries.is_empty() {
        bail!("No instances found");
    }
    let selected = interactive::select_instance(&entries)?;
    println!("Selected instance: {selected}");

    let group = ProfileGroup {
        profile: profile.clone(),
        instance_id: selected.instance_id.clone(),
    };

    match edit_index {
        Some(i) => config.groups[i] = group,
        None => config.groups.push(group),
    }

    config.save()?;
    Ok(())
}

fn main() -> Result<()> {
    signal::install_ctrlc_handler()?;

    let cli = Cli::parse();

    if cli.configure {
        return run_configure();
    }

    let config = AwsUtilConfig::load()?;
    if config.groups.is_empty() {
        println!("No configuration found. Run `aws-util --configure` first.");
        return Ok(());
    }

    let group = if config.groups.len() == 1 {
        &config.groups[0]
    } else {
        interactive::select_group(&config.groups)?
    };

    let client = AwsClient::new(&group.profile);
    let instance = client.instance(&group.instance_id);
    let instance_id = &group.instance_id;

    if let Some(time_str) = cli.schedule_shutdown {
        let (minutes, target_time) = schedule::minutes_until_next(&time_str)?;
        instance.schedule_shutdown(minutes, &target_time.format("%H:%M").to_string())?;
        return Ok(());
    }

    if cli.stop {
        match instance.state()? {
            Some(state) if state == "stopped" => {
                println!(
                    "Instance {instance_id} is already {}.",
                    style("stopped").red()
                );
            }
            _ => {
                instance.stop_and_wait()?;
            }
        }
        return Ok(());
    }

    match instance.state()? {
        Some(state) if state == "running" => {
            println!(
                "Instance {instance_id} is already {}.",
                style("running").green()
            );
        }
        _ => {
            instance.start_and_wait()?;
        }
    }

    Ok(())
}
