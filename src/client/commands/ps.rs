use crate::client::cli::ClientConfig;
use crate::client::request;
use crate::daemon::api::{ApiModuleRunStatus, ApiProbeStatus};
use anyhow::Result;
use chrono::Local;
use std::convert::TryFrom;
use std::io;
use std::io::Write;
use std::time::Duration;
use tabwriter::TabWriter;

pub fn list_modules_cmd(cfg: &ClientConfig) -> Result<()> {
    let module_status = request::list_modules(&cfg.daemon_url)?;
    let mut tw = TabWriter::new(io::stdout()).minwidth(8);

    writeln!(&mut tw, "pid\tname\tliveness\tstatus\tsince")?;
    module_status.status.iter().try_for_each(|mod_status| {
        let formatted_status = match mod_status.status {
            ApiModuleRunStatus::RUNNING => "running",
            ApiModuleRunStatus::STOPPED => "stopped",
            ApiModuleRunStatus::WAITING => "waiting",
            ApiModuleRunStatus::EXITED => "exited",
        };
        let formatted_liveness_status = match mod_status.liveness_status {
            Some(ApiProbeStatus::Pending) => "pending",
            Some(ApiProbeStatus::Successful) => "healthy",
            Some(ApiProbeStatus::Failing) => "failing",
            Some(ApiProbeStatus::Error) => "erroring",
            Some(_) => "none",
            None => "none",
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
            "{}\t{}\t{}\t{}\t{}",
            mod_status.pid,
            mod_status.name,
            formatted_liveness_status,
            formatted_status,
            formatted_time,
        )
    })?;
    tw.flush()?;
    Ok(())
}
