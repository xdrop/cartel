use crate::client::cli::ClientConfig;
use crate::client::config::get_module_by_name;
use crate::client::module::{InnerDefinition, ModuleKind};
use crate::client::request;
use anyhow::{bail, Result};
use std::ffi::OsString;
#[cfg(unix)]
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::Command;

pub enum LogMode {
    FULL,
    FOLLOW,
    DEFAULT,
}

fn get_log_file(module_name: &str, cfg: &ClientConfig) -> Result<OsString> {
    let module = get_module_by_name(module_name, cfg)?;
    // If it is a task with a custom path then use that. Since tasks are
    // stateless there is no reason to contact the daemon.
    if let Some(ref m) = module {
        if let InnerDefinition::Task(tsk) = &m.inner {
            if let Some(path) = &tsk.log_file_path {
                return Ok(OsString::from(path));
            }
        }
    }

    let module_kind = if let Some(m) = module {
        m.kind
    } else {
        // A service may have been removed from the module definitions file. In
        // such case we want to still try to obtain logs for it.
        ModuleKind::Service
    };

    let path =
        request::log_file_path(module_name, &module_kind, &cfg.daemon_url)?
            .log_file_path;
    Ok(path)
}

pub fn print_logs(
    module_name: &str,
    log_mode: LogMode,
    cfg: &ClientConfig,
) -> Result<()> {
    let log_file = get_log_file(module_name, cfg)?;

    if !Path::new(&log_file).exists() {
        bail!("Log file not found for module {}", module_name);
    }

    // This might fail on systems like Windows since paths may not be UTF-8
    // encoded there. Since we are using 'less' to page the logs and we don't
    // support Windows this is not currently an issue, but worth revisiting
    // if support for Windows is to be added.
    let unix_path = log_file
        .to_str()
        .expect("Systems where paths aren't UTF-8 encoded are not supported");

    let pager_cmd = match log_mode {
        LogMode::DEFAULT => &cfg.default_pager_cmd,
        LogMode::FOLLOW => &cfg.follow_pager_cmd,
        LogMode::FULL => &cfg.full_pager_cmd,
    };

    #[cfg(unix)]
    {
        Command::new(&pager_cmd[0])
            .args(&pager_cmd[1..])
            .arg(unix_path)
            .exec(); // Note: The process ends here; subsequent code won't run.
    }
    // This is entirely untested, need to get someone with a Windows machine to
    // test it + set an appropriate pager for Windows.
    #[cfg(windows)]
    {
        Command::new(&pager_cmd[0])
            .args(&pager_cmd[1..])
            .arg(unix_path)
            .spawn()?
            .wait()?;
    }

    Ok(())
}
