use crate::client::cli::CliOptions;
use crate::client::emoji::{HOUR_GLASS, SUCCESS};
use crate::client::request::stop_module;
use console::style;

pub fn stop_module_cmd(module: &str, cli_config: &CliOptions) -> () {
    #[rustfmt::skip]
    tprintstep!(format!("Stopping service '{}'...", module), 1, 2, HOUR_GLASS);
    stop_module(module, &cli_config.daemon_url);
    tprintstep!(style("Service stopped").bold().green(), 2, 2, SUCCESS);
}
