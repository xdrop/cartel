use crate::client::cli::ClientConfig;
use crate::client::emoji::{HOUR_GLASS, SUCCESS};
use crate::client::request;
use anyhow::Result;
use console::style;

pub fn down_cmd(cfg: &ClientConfig) -> Result<()> {
    tprintstep!("Stopping all service(s)...", 1, 2, HOUR_GLASS);
    request::stop_all(&cfg.daemon_url)?;
    tprintstep!(style("Service(s) stopped").bold().green(), 2, 2, SUCCESS);
    Ok(())
}
