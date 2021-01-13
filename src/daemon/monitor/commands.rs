#[derive(Debug)]
pub enum MonitorTask {
    Executable(ExecMonitor),
}

#[derive(Debug)]
pub struct Monitor {
    /// The number of retries before the monitor is considered failed.
    pub retries: u32,
    /// Enum of different monitor task types. They indicate what to perform as
    /// the monitor task.
    pub task: MonitorTask,
}

#[derive(Debug)]
pub struct ExecMonitor {
    pub command: Vec<String>,
    pub working_dir: Option<String>,
}

#[derive(Debug)]
pub enum MonitorCommand {
    NewMonitor { key: String, monitor: Monitor },
    PerformPoll,
}
