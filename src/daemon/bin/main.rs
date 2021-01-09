extern crate cartel;

use cartel::daemon::api;
use cartel::daemon::signal;
use cartel::daemon::Core;
use std::error::Error;
use std::sync::Arc;

pub fn init() {
    env_logger::init()
}

fn main() -> Result<(), Box<dyn Error>> {
    init();
    let core = Arc::new(Core::new());
    signal::setup_signal_handlers(Arc::clone(&core))?;
    api::engine::start(&core);
    Ok(())
}
