use parking_lot::Mutex;
use std::collections::HashMap;

#[derive(Copy, Clone, Debug)]
pub enum MonitorStatus {
    Pending = 0x1,
    Successful = 0x2,
    RetriesExceeded = 0x3,
    Error = 0x4,
    Failing = 0x5,
}

pub struct MonitorState {
    monitor_map: Mutex<HashMap<String, MonitorStatus>>,
}

impl MonitorState {
    pub(super) fn monitor_status(
        &self,
        monitor_name: &str,
    ) -> Option<MonitorStatus> {
        let map = self.monitor_map.lock();
        map.get(monitor_name).copied()
    }

    pub(super) fn monitor_statuses(&self) -> HashMap<String, MonitorStatus> {
        let map = self.monitor_map.lock();
        map.clone()
    }

    pub(super) fn update_states(
        &self,
        new_states: Vec<(String, MonitorStatus)>,
    ) {
        let mut map = self.monitor_map.lock();
        new_states.into_iter().for_each(|(monitor, is_done)| {
            map.insert(monitor, is_done);
        });
    }

    pub fn new() -> MonitorState {
        MonitorState {
            monitor_map: Mutex::new(HashMap::new()),
        }
    }
}

impl Default for MonitorState {
    fn default() -> Self {
        Self::new()
    }
}
