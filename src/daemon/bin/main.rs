extern crate cartel;

use cartel::daemon::api;
use cartel::daemon::monitor;
use cartel::daemon::signal;
use cartel::daemon::Core;
use std::error::Error;
use std::sync::Arc;

pub fn init() {
    env_logger::init()
}

fn main() -> Result<(), Box<dyn Error>> {
    init();
    let monitor = monitor::MonitorState::new();
    let monitor_handle = monitor::spawn_runtime(Arc::new(monitor));
    let core = Arc::new(Core::new(monitor_handle));
    signal::setup_signal_handlers(Arc::clone(&core))?;
    api::engine::start(&core);
    Ok(())
}
