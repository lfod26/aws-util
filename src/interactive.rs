use anyhow::{Context, Result};
use dialoguer::{FuzzySelect, theme::ColorfulTheme};

use crate::config::ProfileGroup;
use crate::ec2::InstanceEntry;

/// Prompts the user to fuzzy-search and pick one AWS CLI profile from the
/// given list (as returned by `aws configure list-profiles`).
pub fn select_profile(profiles: &[String]) -> Result<&String> {
    let selection = FuzzySelect::with_theme(&ColorfulTheme::default())
        .with_prompt("Search for an AWS profile (type to filter)")
        .items(profiles)
        .default(0)
        .interact()
        .context("failed to read selection")?;

    Ok(&profiles[selection])
}

/// Prompts the user to fuzzy-search and pick one instance from the list.
pub fn select_instance(entries: &[InstanceEntry]) -> Result<&InstanceEntry> {
    let selection = FuzzySelect::with_theme(&ColorfulTheme::default())
        .with_prompt("Search for an instance (type to filter)")
        .items(entries)
        .default(0)
        .interact()
        .context("failed to read selection")?;

    Ok(&entries[selection])
}

/// Prompts the user to fuzzy-search and pick which configured
/// profile/instance group to act on. Only meaningful when more than one
/// group is configured (callers should skip this and use the single
/// group directly otherwise).
pub fn select_group(groups: &[ProfileGroup]) -> Result<&ProfileGroup> {
    let selection = FuzzySelect::with_theme(&ColorfulTheme::default())
        .with_prompt("Select a profile/instance to act on")
        .items(groups)
        .default(0)
        .interact()
        .context("failed to read selection")?;

    Ok(&groups[selection])
}

/// Prompts the user to either pick an existing profile/instance group to
/// replace, or add a new one, during `--configure`. Returns the index of
/// the group to replace, or `None` if the user chose to add a new one.
pub fn select_group_to_edit(groups: &[ProfileGroup]) -> Result<Option<usize>> {
    const ADD_NEW: &str = "+ Add new profile/instance";

    let mut items: Vec<String> = groups.iter().map(ProfileGroup::to_string).collect();
    items.push(ADD_NEW.to_string());

    let selection = FuzzySelect::with_theme(&ColorfulTheme::default())
        .with_prompt("Select a profile/instance to edit, or add a new one")
        .items(&items)
        .default(0)
        .interact()
        .context("failed to read selection")?;

    Ok((selection < groups.len()).then_some(selection))
}
