use anyhow::{Context, Result};

/// Installs a Ctrl+C handler that restores the terminal (cursor visibility
/// and raw mode, which `dialoguer` prompts may have enabled) before exiting,
/// so a Ctrl+C press during an interactive prompt or a long-running wait
/// doesn't leave the terminal in a broken state or surface Windows'
/// STATUS_CONTROL_C_EXIT as an abrupt error.
pub fn install_ctrlc_handler() -> Result<()> {
    ctrlc::set_handler(|| {
        let term = dialoguer::console::Term::stdout();
        let _ = term.show_cursor();
        eprintln!("\nInterrupted (Ctrl+C). Exiting.");
        std::process::exit(130);
    })
    .context("failed to set Ctrl+C handler")
}
