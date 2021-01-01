use crate::daemon::module::ModuleKind;
use dirs::home_dir;
use std::fs;
use std::path::PathBuf;

const PROJECT_DIR: &str = ".cartel";
const LOG_DIR: &str = "logs";

/// Returns the default log directory as a `PathBuf`.
pub fn default_log_directory() -> PathBuf {
    let home_dir = home_dir()
        .expect("Failed to get home dir")
        .join(PROJECT_DIR)
        .join(LOG_DIR);
    // TODO: Handle
    fs::create_dir_all(home_dir.as_path()).expect("Failed to create log dir");
    home_dir
}

/// Returns the log file path for a given module name.
///
/// The log path is differentiated based on the module kind. For example a
/// service will get a different log file than a task with the same name.
pub fn log_file_path(module_name: &str, module_kind: &ModuleKind) -> PathBuf {
    let base = default_log_directory();
    match module_kind {
        ModuleKind::Task => base.join(format!("{}.log", module_name)),
        ModuleKind::Service => base.join(format!("{}.task.log", module_name)),
    }
}
