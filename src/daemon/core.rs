use crate::daemon::monitor::{self, MonitorHandle};
use crate::daemon::planner::Planner;
use crate::daemon::{api, signal};

use std::error::Error;
use std::sync::Arc;

/// Holds the core daemon state.
pub struct Core {
    pub planner: Planner,
}

impl Core {
    // Note: We can potentially remove this wrapper struct if there isn't a
    // solid use case for having more than a planner in this. All interactions
    // can go directly to the planner.

    /// Initializes the daemon core.
    ///
    /// Most interaction with the daemon will happen through the [Planner]
    /// instance held in the core.
    pub fn new(monitor_handle: MonitorHandle) -> Core {
        Core {
            planner: Planner::new(monitor_handle),
        }
    }

    /// Return a reference to the planner.
    pub fn planner(&self) -> &Planner {
        &self.planner
    }
}

/// Start the daemon
pub fn start_daemon() -> Result<(), Box<dyn Error>> {
    let monitor = monitor::MonitorState::new();

    // Create the Tokio async runtime and pass a handle to it so that it can be
    // invoked from a sync context from within the API handlers.
    let monitor_handle = monitor::spawn_runtime(Arc::new(monitor));
    let core = Arc::new(Core::new(monitor_handle));

    // Setup signal handlers to collect dead child processes.
    signal::setup_signal_handlers(Arc::clone(&core))?;

    // Start the API.
    api::engine::start(&core);

    Ok(())
}
