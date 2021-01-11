use crate::client::cli::CliOptions;
use crate::client::emoji::{HOUR_GLASS, SUCCESS};
use crate::client::request;
use anyhow::Result;
use console::style;

pub fn down_cmd(cli_config: &CliOptions) -> Result<()> {
    tprintstep!(format!("Stopping all service(s)..."), 1, 2, HOUR_GLASS);
    request::stop_all(&cli_config.daemon_url)?;
    tprintstep!(style("Service(s) stopped").bold().green(), 2, 2, SUCCESS);
    Ok(())
}
