use crate::daemon::{
    api::{
        ApiExeProbe, ApiKind, ApiLogLineProbe, ApiNetworkProbe, ApiProbe,
        ApiTermSignal,
    },
    planner::MonitorStatus,
};
use crate::{
    client::module::{ModuleKind, Probe, TermSignal},
    daemon::api::ApiProbeStatus,
};

impl From<&ModuleKind> for ApiKind {
    fn from(kind: &ModuleKind) -> ApiKind {
        match kind {
            ModuleKind::Service => ApiKind::Service,
            ModuleKind::Task => ApiKind::Task,
            ModuleKind::Check => ApiKind::Task,
            ModuleKind::Group => ApiKind::Task,
            ModuleKind::Shell => ApiKind::Task,
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
                command: exec.command.clone(),
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
