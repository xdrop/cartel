use super::handlers::{ApiModuleDefinition, ApiModuleRunStatus};
use crate::daemon::executor::RunStatus;
use crate::daemon::module::{ModuleDefinition, ModuleKind};
use std::path::PathBuf;

pub fn from_task(src: ApiModuleDefinition) -> ModuleDefinition {
    ModuleDefinition::new(
        ModuleKind::Task,
        src.name,
        src.command,
        src.environment,
        src.log_file_path,
        src.dependencies,
        src.working_dir.map(PathBuf::from),
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
        src.working_dir.map(PathBuf::from),
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
