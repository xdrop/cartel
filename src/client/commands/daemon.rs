use crate::client::emoji::{HOUR_GLASS, SUCCESS};
use crate::shell::active_shell_path;
use anyhow::{bail, Result};
use console::style;
use std::process::Command;

pub fn restart_daemon() -> Result<()> {
    let active_shell = if let Some(path) = active_shell_path() {
        path
    } else {
        bail!("Unsupported shell")
    };

    tprintstep!("Restarting daemon...", 1, 2, HOUR_GLASS);
    let cmd_line = [
        active_shell.as_str(),
        "-c",
        "pkill -i cartel-daemon; cartel-daemon &",
    ];

    let mut cmd = Command::new(cmd_line[0]);
    let status = cmd.args(&cmd_line[1..]).spawn()?.wait()?;
    if !status.success() {
        bail!(
            "Failed to restart. This feature only works if \
            'pkill' is installed on your system."
        );
    }
    tprintstep!(style("Daemon restarted").bold().green(), 2, 2, SUCCESS);
    Ok(())
}
