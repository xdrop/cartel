extern crate cartel;
use std::error::Error;

use cartel::daemon::bootstrap::bootstrap_shell;
use cartel::daemon::cli::cli_app;
use cartel::daemon::core::start_daemon;
use cartel::detach::detach_tty;

pub fn init() {
    env_logger::init()
}

fn main() -> Result<(), Box<dyn Error>> {
    init();
    let config = cli_app()?;
    #[cfg(unix)]
    {
        if config.detach_tty {
            let args = std::env::args();
            detach_tty(args, false);
        }
        // Re-run the daemon in a shell
        if let Some(shell_path) = config.shell {
            bootstrap_shell(shell_path);
        }
    }
    start_daemon()?;
    Ok(())
}
