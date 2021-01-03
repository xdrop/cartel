use crate::daemon::planner::Planner;

pub struct Core {
    pub planner: Planner,
}

impl Core {
    pub fn new() -> Core {
        Core {
            planner: Planner::new(),
        }
    }

    pub fn planner(&self) -> &Planner {
        &self.planner
    }
}

impl Default for Core {
    fn default() -> Self {
        Self::new()
    }
}
