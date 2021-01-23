use crate::client::cli::ClientConfig;
use crate::client::emoji::{HOUR_GLASS, SUCCESS};
use crate::client::request;
use anyhow::Result;
use console::style;

pub fn restart_module_cmd(module: &str, cfg: &ClientConfig) -> Result<()> {
    #[rustfmt::skip]
    tprintstep!(format!("Restarting service '{}'...", module), 1, 2, HOUR_GLASS);
    request::restart_module(module, &cfg.daemon_url)?;
    tprintstep!(style("Service restarted").bold().green(), 2, 2, SUCCESS);
    Ok(())
}
