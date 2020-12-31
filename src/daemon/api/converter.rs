use super::super::executor::RunStatus;
use super::super::module::ModuleDefinition;
use super::handlers::{ApiModuleDefinition, ApiModuleRunStatus};
use anyhow::Result;
use std::path::PathBuf;

pub fn to_mod_def(src: ApiModuleDefinition) -> ModuleDefinition {
    ModuleDefinition::new(
        src.name,
        src.command,
        src.environment,
        src.log_file_path,
        src.dependencies,
        src.working_dir.map(|dir| PathBuf::from(dir)),
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
