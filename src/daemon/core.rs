use crate::daemon::monitor::MonitorHandle;
use crate::daemon::planner::Planner;

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
