use crate::client::cli::ClientConfig;
use crate::client::request;
use anyhow::Result;
#[cfg(unix)]
use std::os::unix::process::CommandExt;
use std::process::Command;

pub enum LogMode {
    FULL,
    FOLLOW,
    DEFAULT,
}

pub fn print_logs(
    module_name: &str,
    log_mode: LogMode,
    cfg: &ClientConfig,
) -> Result<()> {
    let log_file = request::log_info(module_name, &cfg.daemon_url)?;

    // This might fail on systems like Windows since paths may not be UTF-8
    // encoded there. Since we are using 'less' to page the logs and we don't
    // support Windows this is not currently an issue, but worth revisiting
    // if support for Windows is to be added.
    let unix_path = log_file
        .log_file_path
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
