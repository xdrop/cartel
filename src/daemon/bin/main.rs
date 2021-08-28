extern crate cartel;
use std::error::Error;

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
    }
    start_daemon()?;
    Ok(())
}
