use crate::client::cli::ClientConfig;
use crate::client::request;
use crate::daemon::api::{ApiModuleRunStatus, ApiProbeStatus};
use anyhow::Result;
use chrono::Local;
use clap::ArgMatches;
use console::Style;
use std::convert::TryFrom;
use std::io;
use std::io::Write;
use std::time::Duration;
use tabwriter::TabWriter;

pub struct PsOpts {
    pub color: bool,
}

impl PsOpts {
    pub fn from(matches: &ArgMatches) -> Self {
        Self {
            color: !matches.is_present("no-color"),
        }
    }
}

fn get_header_style(ps_opts: &PsOpts) -> Style {
    if ps_opts.color {
        Style::new().bold()
    } else {
        Style::new()
    }
}

fn get_line_style(
    ps_opts: &PsOpts,
    run_status: ApiModuleRunStatus,
    probe_status: ApiProbeStatus,
) -> Style {
    if ps_opts.color {
        if run_status == ApiModuleRunStatus::RUNNING
            && probe_status == ApiProbeStatus::Successful
        {
            console::Style::new()
        } else if (run_status == ApiModuleRunStatus::STOPPED
            || run_status == ApiModuleRunStatus::WAITING)
            && (probe_status == ApiProbeStatus::Pending
                || probe_status == ApiProbeStatus::Successful)
        {
            console::Style::new().dim()
        } else {
            console::Style::new().red()
        }
    } else {
        console::Style::new()
    }
}

pub fn list_modules_cmd(ps_opts: &PsOpts, cfg: &ClientConfig) -> Result<()> {
    let module_status = request::list_modules(&cfg.daemon_url)?;
    let mut tw = TabWriter::new(io::stdout()).minwidth(8);

    writeln!(
        &mut tw,
        "{}",
        get_header_style(ps_opts)
            .apply_to("pid\tname\tliveness\tstatus\tsince")
    )?;

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
            Some(_) => "-",
            None => "-",
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

        let liveness_status = mod_status
            .liveness_status
            .clone()
            .unwrap_or(ApiProbeStatus::Successful);

        writeln!(
            &mut tw,
            "{}",
            get_line_style(ps_opts, mod_status.status, liveness_status)
                .apply_to(format!(
                    "{}\t{}\t{}\t{}\t{}",
                    mod_status.pid,
                    mod_status.name,
                    formatted_liveness_status,
                    formatted_status,
                    formatted_time,
                ))
        )
    })?;
    tw.flush()?;
    Ok(())
}
