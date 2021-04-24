use crate::client::cli::ClientConfig;
use crate::client::emoji::{HOUR_GLASS, SUCCESS};
use crate::client::progress::{SpinnerOptions, WaitResult, WaitUntil};
use crate::client::request;
use anyhow::Result;
use console::style;

pub fn stop_service_cmd(services: Vec<&str>, cfg: &ClientConfig) -> Result<()> {
    tprintstep!("Stopping service(s)...", 1, 2, HOUR_GLASS);
    for service in services {
        stop_service(service, cfg)?;
    }
    tprintstep!(style("Service(s) stopped").bold().green(), 2, 2, SUCCESS);
    Ok(())
}

fn stop_service(service: &str, cfg: &ClientConfig) -> Result<()> {
    let message = format!("Stopping {}", style(service).white().bold());
    let spin_opt = SpinnerOptions::new(message).clear_on_finish(false);

    let wu = WaitUntil::new(&spin_opt);
    wu.spin_until_status(|| {
        let status = style("(Stopped)").white().dim().bold().to_string();
        request::stop_module(service, &cfg.daemon_url)?;
        Ok(WaitResult::from((), status))
    })?;

    Ok(())
}
