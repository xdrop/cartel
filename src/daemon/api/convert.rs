use super::handlers::*;
use crate::daemon::executor::RunStatus;
use crate::daemon::logs::log_file_module;
use crate::daemon::module::{ModuleDefinition, ModuleKind, TermSignal};
use crate::daemon::monitor::{
    ExecMonitor, LogLineMonitor, Monitor, MonitorTask, NetMonitor,
};
use crate::daemon::planner::{Plan, PlannedAction};
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
        None,
    )
}

pub fn from_service_with_monitor(
    mut src: ApiModuleDefinition,
) -> (ModuleDefinition, Option<Monitor>) {
    let readiness_probe = src.readiness_probe.take();
    let liveness_probe = src.liveness_probe.take();

    let mut module_definition = ModuleDefinition::new(
        ModuleKind::Service,
        src.name,
        src.command,
        src.environment,
        src.log_file_path,
        src.dependencies,
        src.working_dir.and_then(path::from_user_string),
        src.termination_signal.into(),
        None, // assigned below
    );

    let log_file_path = log_file_module(&module_definition);

    let readiness_monitor: Option<Monitor> = match readiness_probe {
        Some(probe) => Some(from_probe(probe, &log_file_path)),
        None => None,
    };
    let liveness_monitor: Option<Monitor> = match liveness_probe {
        Some(probe) => Some(from_probe(probe, &log_file_path)),
        None => None,
    };

    // Only store liveness as readiness only affects the service temporarily
    module_definition.liveness_probe = liveness_monitor;

    (module_definition, readiness_monitor)
}

pub fn from_task_or_service(src: ApiModuleDefinition) -> ModuleDefinition {
    ModuleDefinition::new(
        src.kind.into(),
        src.name,
        src.command,
        src.environment,
        src.log_file_path,
        src.dependencies,
        src.working_dir.and_then(path::from_user_string),
        src.termination_signal.into(),
        None, // assumed not needed in any code using this
    )
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

impl From<ApiModuleKind> for ModuleKind {
    fn from(src: ApiModuleKind) -> Self {
        match src {
            ApiModuleKind::Service => ModuleKind::Service,
            ApiModuleKind::Task => ModuleKind::Task,
        }
    }
}

impl From<PlannedAction> for ApiPlannedAction {
    fn from(src: PlannedAction) -> Self {
        match src {
            PlannedAction::WillDeploy => ApiPlannedAction::WillDeploy,
            PlannedAction::AlreadyDeployed => ApiPlannedAction::AlreadyDeployed,
            PlannedAction::WillRedeploy => ApiPlannedAction::WillRedeploy,
        }
    }
}

impl From<Plan> for ApiGetPlanResponse {
    fn from(mut src: Plan) -> Self {
        ApiGetPlanResponse {
            plan: src.plan.drain().map(|(k, v)| (k, v.into())).collect(),
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

pub fn from_probe(probe: ApiProbe, log_file_path: &Path) -> Monitor {
    match probe {
        ApiProbe::Executable(exe) => exe.into(),
        ApiProbe::LogLine(log) => from_log_line_probe(log, log_file_path),
        ApiProbe::Net(net) => net.into(),
    }
}

impl From<ApiExeProbe> for Monitor {
    fn from(exe: ApiExeProbe) -> Monitor {
        Monitor {
            retries: exe.retries,
            task: MonitorTask::Executable(ExecMonitor::from(
                exe.command,
                exe.working_dir,
            )),
        }
    }
}

impl From<ApiNetworkProbe> for Monitor {
    fn from(net: ApiNetworkProbe) -> Monitor {
        Monitor {
            retries: net.retries,
            task: MonitorTask::Net(NetMonitor::from(net.hostname, net.port)),
        }
    }
}

pub fn from_log_line_probe(
    log_line: ApiLogLineProbe,
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
