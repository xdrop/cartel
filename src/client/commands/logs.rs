use crate::client::cli::CliOptions;
use crate::client::request::log_info;
use anyhow::Result;
#[cfg(unix)]
use std::os::unix::process::CommandExt;
use std::process::Command;

pub fn print_logs(module_name: &str, cli_config: &CliOptions) -> Result<()> {
    let log_file = log_info(module_name, &cli_config.daemon_url)?;

    // This might fail on systems like Windows since paths may not be UTF-8
    // encoded there. Since we are using 'less' to page the logs and we don't
    // support Windows this is not currently an issue, but worth revisiting
    // if support for Windows is to be added.
    let unix_path = log_file
        .log_file_path
        .to_str()
        .expect("Systems where paths aren't UTF-8 encoded are not supported");

    #[cfg(unix)]
    {
        Command::new(&cli_config.pager_cmd[0])
            .args(&cli_config.pager_cmd[1..])
            .arg(unix_path)
            .exec(); // Note: The process ends here; subsequent code won't run.
    }
    #[cfg(windows)]
    {
        Command::new(&cli_config.pager_cmd[0])
            .args(&cli_config.pager_cmd[1..])
            .arg(unix_path)
            .spawn()?
            .wait()?;
    }

    Ok(())
}
