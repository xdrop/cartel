use crate::client::cli::CliOptions;
use crate::client::request;
use crate::daemon::api::ApiModuleRunStatus;
use anyhow::Result;
use chrono::Local;
use std::convert::TryFrom;
use std::io;
use std::io::Write;
use std::time::Duration;
use tabwriter::TabWriter;

pub fn list_modules_cmd(cli_config: &CliOptions) -> Result<()> {
    let module_status = request::list_modules(&cli_config.daemon_url)?;
    let mut tw = TabWriter::new(io::stdout()).minwidth(8);

    writeln!(&mut tw, "pid\tname\tstatus\tsince")?;
    module_status.status.iter().try_for_each(|mod_status| {
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

        writeln!(
            &mut tw,
            "{}\t{}\t{}\t{}",
            mod_status.pid, mod_status.name, formatted_status, formatted_time,
        )
    })?;
    tw.flush()?;
    Ok(())
}
