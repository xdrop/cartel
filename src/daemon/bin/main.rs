extern crate cartel;
use std::error::Error;

use cartel::daemon::bootstrap::bootstrap_shell;
use cartel::daemon::cli::cli_app;
use cartel::daemon::core::start_daemon;

pub fn init() {
    env_logger::init()
}

fn main() -> Result<(), Box<dyn Error>> {
    init();
    let config = cli_app()?;
    #[cfg(unix)]
    {
        // Re-run the daemon in a shell
        if let Some(shell_path) = config.shell {
            bootstrap_shell(shell_path);
        }
    }
    start_daemon()?;
    Ok(())
}
