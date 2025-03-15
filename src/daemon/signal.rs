use crate::daemon::Core;
use log::info;
use signal_hook::iterator::Signals;
use signal_hook::{SIGCHLD, SIGINT, SIGTERM};
use std::error::Error;
use std::sync::Arc;

pub fn setup_signal_handlers(core: Arc<Core>) -> Result<(), Box<dyn Error>> {
    let signals = Signals::new([SIGCHLD, SIGTERM, SIGINT])?;

    std::thread::spawn(move || {
        for sig in signals.forever() {
            if sig == SIGCHLD {
                info!("Running collection of dead children...");
                core.planner().collect_dead();
            } else if sig == SIGTERM || sig == SIGINT {
                info!("Cleaning up and exiting...");
                core.planner().cleanup().ok();
                std::process::exit(0);
            }
        }
    });
    Ok(())
}
