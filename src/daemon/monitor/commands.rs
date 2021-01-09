#[derive(Debug)]
pub enum Monitor {
    Executable(ExeMonitor),
}

#[derive(Debug)]
pub struct ExeMonitor {
    pub command: Vec<String>,
    pub working_dir: Option<String>,
}

#[derive(Debug)]
pub enum MonitorCommand {
    NewMonitor { key: String, monitor: Monitor },
    PerformPoll,
}
