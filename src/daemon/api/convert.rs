use super::handlers::*;
use crate::daemon::logs::log_file_module;
use crate::daemon::module::{ModuleDefinition, ModuleKind, TermSignal};
use crate::daemon::monitor::{
    ExecMonitor, LogLineMonitor, Monitor, MonitorTask,
};
use crate::daemon::{executor::RunStatus, monitor::NetMonitor};
use crate::path;
use std::path::Path;

pub fn from_task(src: ApiModuleDefinition) -> ModuleDefinition {
    ModuleDefinition::new(
        ModuleKind::Task,
        src.name,
        src.command,
        src.environment,
        src.log_file_path,
        src.dependencies,
        src.working_dir.and_then(path::from_user_string),
        TermSignal::KILL,
    )
}

pub fn from_service(
    mut src: ApiModuleDefinition,
) -> (ModuleDefinition, Option<Monitor>) {
    let healthcheck = src.healthcheck.take();

    let module_definition = ModuleDefinition::new(
        ModuleKind::Service,
        src.name,
        src.command,
        src.environment,
        src.log_file_path,
        src.dependencies,
        src.working_dir.and_then(path::from_user_string),
        src.termination_signal.into(),
    );

    let monitor: Option<Monitor> = match healthcheck {
        Some(ApiHealthcheck::Executable(exe)) => Some(exe.into()),
        Some(ApiHealthcheck::LogLine(log_line)) => {
            Some(from_log_line_healthcheck(
                log_line,
                &log_file_module(&module_definition),
            ))
        }
        Some(ApiHealthcheck::Net(net)) => Some(net.into()),
        None => None,
    };

    (module_definition, monitor)
}

impl From<RunStatus> for ApiModuleRunStatus {
    fn from(r: RunStatus) -> ApiModuleRunStatus {
        match r {
            RunStatus::RUNNING => ApiModuleRunStatus::RUNNING,
            RunStatus::STOPPED => ApiModuleRunStatus::STOPPED,
            RunStatus::WAITING => ApiModuleRunStatus::WAITING,
            RunStatus::EXITED => ApiModuleRunStatus::EXITED,
        }
    }
}

impl From<ApiTermSignal> for TermSignal {
    fn from(signal: ApiTermSignal) -> TermSignal {
        match signal {
            ApiTermSignal::TERM => TermSignal::TERM,
            ApiTermSignal::KILL => TermSignal::KILL,
            ApiTermSignal::INT => TermSignal::INT,
        }
    }
}

impl From<ApiExeHealthcheck> for Monitor {
    fn from(exe: ApiExeHealthcheck) -> Monitor {
        Monitor {
            retries: exe.retries,
            task: MonitorTask::Executable(ExecMonitor::from(
                exe.command,
                exe.working_dir,
            )),
        }
    }
}

impl From<ApiNetworkHealthcheck> for Monitor {
    fn from(net: ApiNetworkHealthcheck) -> Monitor {
        Monitor {
            retries: net.retries,
            task: MonitorTask::Net(NetMonitor::from(net.hostname, net.port)),
        }
    }
}

pub fn from_log_line_healthcheck(
    log_line: ApiLogLineHealthcheck,
    log_file_path: &Path,
) -> Monitor {
    Monitor {
        retries: log_line.retries,
        task: MonitorTask::LogLine(LogLineMonitor::from(
            log_line.line_regex,
            log_file_path,
        )),
    }
}
