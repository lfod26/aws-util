use anyhow::{Context, Result};
use dialoguer::{FuzzySelect, Input, theme::ColorfulTheme};

use crate::ec2::InstanceEntry;

/// Prompts the user to type in an AWS profile name (plain text, not
/// selectable from a list).
pub fn prompt_profile() -> Result<String> {
    Input::with_theme(&ColorfulTheme::default())
        .with_prompt("AWS profile name")
        .interact_text()
        .context("failed to read profile name")
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
