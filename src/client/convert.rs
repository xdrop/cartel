use crate::client::module::{ModuleKind, Probe, TermSignal};
use crate::daemon::api::{
    ApiExeProbe, ApiLogLineProbe, ApiModuleKind, ApiNetworkProbe, ApiProbe,
    ApiProbeStatus, ApiTermSignal,
};
use crate::daemon::planner::MonitorStatus;

impl From<&ModuleKind> for ApiModuleKind {
    fn from(kind: &ModuleKind) -> ApiModuleKind {
        match kind {
            ModuleKind::Service => ApiModuleKind::Service,
            ModuleKind::Task => ApiModuleKind::Task,
            ModuleKind::Check => ApiModuleKind::Task,
            ModuleKind::Group => ApiModuleKind::Task,
            ModuleKind::Shell => ApiModuleKind::Task,
        }
    }
}

impl From<&TermSignal> for ApiTermSignal {
    fn from(signal: &TermSignal) -> ApiTermSignal {
        match signal {
            TermSignal::TERM => ApiTermSignal::TERM,
            TermSignal::KILL => ApiTermSignal::KILL,
            TermSignal::INT => ApiTermSignal::INT,
        }
    }
}

impl From<&MonitorStatus> for ApiProbeStatus {
    fn from(status: &MonitorStatus) -> Self {
        match status {
            MonitorStatus::Error => Self::Error,
            MonitorStatus::Failing => Self::Failing,
            MonitorStatus::RetriesExceeded => Self::RetriesExceeded,
            MonitorStatus::Successful => Self::Successful,
            MonitorStatus::Pending => Self::Pending,
        }
    }
}

impl From<&Probe> for ApiProbe {
    fn from(probe: &Probe) -> ApiProbe {
        match probe {
            Probe::Exec(exec) => ApiProbe::Executable(ApiExeProbe {
                retries: exec.retries,
                command: exec.cmd_line(),
                working_dir: exec.working_dir.clone(),
            }),
            Probe::LogLine(log_line) => ApiProbe::LogLine(ApiLogLineProbe {
                retries: log_line.retries,
                line_regex: log_line.line_regex.clone(),
            }),
            Probe::Net(net) => ApiProbe::Net(ApiNetworkProbe {
                retries: net.retries,
                hostname: net.host.clone(),
                port: net.port,
            }),
        }
    }
}
