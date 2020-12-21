use crate::daemon::planner::Planner;
use std::sync::{Mutex, MutexGuard};

pub struct Core {
    planner: Mutex<Planner>,
}

impl Core {
    pub fn new() -> Core {
        Core {
            planner: Mutex::new(Planner::new()),
        }
    }

    pub fn planner(&self) -> MutexGuard<Planner> {
        self.planner.lock().expect("Failed to obtain lock")
    }
}
