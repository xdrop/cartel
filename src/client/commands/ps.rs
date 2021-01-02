use crate::client::cli::CliOptions;
use crate::client::request::list_modules;
use crate::daemon::api::ApiModuleRunStatus;
use anyhow::Result;
use chrono::Local;
use std::convert::TryFrom;
use std::time::Duration;

pub fn list_modules_cmd(cli_config: &CliOptions) -> Result<()> {
    let module_status = list_modules(&cli_config.daemon_url)?;

    println!("{:<8}{:<12}{:<12}{:<8}", "pid", "name", "status", "since");
    module_status.status.iter().for_each(|mod_status| {
        let formatted_status = match mod_status.status {
            ApiModuleRunStatus::RUNNING => "running",
            ApiModuleRunStatus::STOPPED => "stopped",
            ApiModuleRunStatus::WAITING => "waiting",
            ApiModuleRunStatus::EXITED => "exited",
        };
        let time_formatter = timeago::Formatter::new();
        let now = u64::try_from(Local::now().timestamp()).unwrap();
        let dur = Duration::new(now - mod_status.time_since_status, 0);
        let formatted_time = if mod_status.status == ApiModuleRunStatus::WAITING
        {
            String::from("N/A")
        } else {
            time_formatter.convert(dur)
        };

        println!(
            "{:<8}{:<12}{:<12}{:<8}",
            mod_status.pid, mod_status.name, formatted_status, formatted_time
        );
    });
    Ok(())
}
