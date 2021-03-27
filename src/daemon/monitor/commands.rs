use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub enum MonitorTask {
    Executable(ExecMonitor),
    LogLine(LogLineMonitor),
    Net(NetMonitor),
}

#[derive(Debug, Clone)]
pub struct Monitor {
    /// The number of retries before the monitor is considered failed.
    pub retries: u32,
    /// Enum of different monitor task types. They indicate what to perform as
    /// the monitor task.
    pub task: MonitorTask,
}

#[derive(Debug, Clone)]
pub struct ExecMonitor {
    pub command: Vec<String>,
    pub working_dir: Option<String>,
}

#[derive(Debug, Clone)]
pub struct NetMonitor {
    pub hostname: String,
    pub port: u16,
}

impl NetMonitor {
    pub fn from(hostname: String, port: u16) -> Self {
        Self { hostname, port }
    }
}

impl ExecMonitor {
    pub fn from(command: Vec<String>, working_dir: Option<String>) -> Self {
        Self {
            command,
            working_dir,
        }
    }
}

#[derive(Debug, Clone)]
pub struct LogLineMonitor {
    pub line_regex: String,
    pub file_path: PathBuf,
}

impl LogLineMonitor {
    pub fn from(line_regex: String, file_path: &Path) -> Self {
        Self {
            line_regex,
            file_path: file_path.to_path_buf(),
        }
    }
}

#[derive(Debug)]
pub enum MonitorType {
    Readiness,
    Liveness,
}

#[derive(Debug)]
pub enum MonitorCommand {
    NewMonitor {
        key: String,
        monitor: Monitor,
        monitor_type: MonitorType,
    },
    RemoveMonitor {
        key: String,
        monitor_type: MonitorType,
    },
    PollReadinessCheck,
    PollLivenessCheck,
    CleanupIdleMonitors,
}
