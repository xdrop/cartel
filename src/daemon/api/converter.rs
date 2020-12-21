use super::super::executor::RunStatus;
use super::super::module::ModuleDefinition;
use super::handlers::{ApiModuleDefinition, ApiModuleRunStatus};

pub fn to_mod_def(src: ApiModuleDefinition) -> ModuleDefinition {
    ModuleDefinition::new(
        src.name,
        src.command,
        src.environment,
        src.log_file_path,
        src.dependencies,
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
