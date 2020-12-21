extern crate cartel;

use cartel::daemon::api;
use cartel::daemon::Core;
use log::info;
use signal_hook::{iterator::Signals, SIGCHLD};
use std::error::Error;
use std::sync::Arc;

pub fn init() -> () {
    env_logger::init()
}

fn main() -> Result<(), Box<dyn Error>> {
    init();
    let signals = Signals::new(&[SIGCHLD])?;
    let core = Arc::new(Core::new());

    let thread_core = Arc::clone(&core);
    std::thread::spawn(move || {
        for _ in signals.forever() {
            info!("Running collection of dead children...");
            thread_core.planner().collect_dead();
        }
    });

    api::engine::start(&core);
    Ok(())
}
