use crate::daemon::monitor::MonitorHandle;
use crate::daemon::planner::Planner;

pub struct Core {
    pub planner: Planner,
}

impl Core {
    pub fn new(monitor_handle: MonitorHandle) -> Core {
        Core {
            planner: Planner::new(monitor_handle),
        }
    }

    pub fn planner(&self) -> &Planner {
        &self.planner
    }
}
