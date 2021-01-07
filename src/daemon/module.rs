use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;

/// The type of the module.
#[derive(Debug, PartialEq)]
pub enum ModuleKind {
    /// A task is a module with a limited lifetime, used to perform some
    /// temporary operation or some setup.
    Task,
    /// A service is a longer running module. It's lifetime will be managed and
    /// can be started, stopped independently.
    Service,
}

/// The choice of terminating signal to use when terminating the process.
///
/// Note: Only implemented for Unix based systems.
#[derive(Debug, PartialEq, Clone)]
pub enum TermSignal {
    /// Translates to SIGKILL on Unix based systems.
    KILL,
    /// Translates to SIGTERM on Unix based systems.
    TERM,
    /// Translates to SIGINT on Unix based systems.
    INT,
}

#[derive(Debug)]
pub struct ModuleDefinition {
    pub kind: ModuleKind,
    pub name: String,
    pub command: Vec<String>,
    pub environment: HashMap<String, String>,
    pub log_file_path: Option<String>,
    pub dependencies: Vec<String>,
    pub working_dir: Option<PathBuf>,
    pub termination_signal: TermSignal,
}

impl Hash for ModuleDefinition {
    fn hash<S: Hasher>(&self, state: &mut S) {
        self.name.hash(state);
    }
}

impl PartialEq for ModuleDefinition {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}

impl ModuleDefinition {
    pub fn new(
        kind: ModuleKind,
        name: String,
        command: Vec<String>,
        environment: HashMap<String, String>,
        log_file_path: Option<String>,
        dependencies: Vec<String>,
        working_dir: Option<PathBuf>,
        termination_signal: TermSignal,
    ) -> ModuleDefinition {
        ModuleDefinition {
            kind,
            name,
            command,
            environment,
            log_file_path,
            dependencies,
            working_dir,
            termination_signal,
        }
    }
}
