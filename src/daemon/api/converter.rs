use super::handlers::{ApiModuleDefinition, ApiModuleRunStatus, ApiTermSignal};
use crate::daemon::executor::RunStatus;
use crate::daemon::module::{ModuleDefinition, ModuleKind, TermSignal};
use crate::path;

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

pub fn from_service(src: ApiModuleDefinition) -> ModuleDefinition {
    ModuleDefinition::new(
        ModuleKind::Service,
        src.name,
        src.command,
        src.environment,
        src.log_file_path,
        src.dependencies,
        src.working_dir.and_then(path::from_user_string),
        src.termination_signal.into(),
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

impl From<ApiTermSignal> for TermSignal {
    fn from(signal: ApiTermSignal) -> TermSignal {
        match signal {
            ApiTermSignal::TERM => TermSignal::TERM,
            ApiTermSignal::KILL => TermSignal::KILL,
            ApiTermSignal::INT => TermSignal::INT,
        }
    }
}
