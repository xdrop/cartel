use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;

#[derive(Debug)]
pub struct ModuleDefinition {
    pub name: String,
    pub command: Vec<String>,
    pub environment: HashMap<String, String>,
    pub log_file_path: Option<String>,
    pub dependencies: Vec<String>,
    pub working_dir: Option<PathBuf>,
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
        name: String,
        command: Vec<String>,
        environment: HashMap<String, String>,
        log_file_path: Option<String>,
        dependencies: Vec<String>,
        working_dir: Option<PathBuf>,
    ) -> ModuleDefinition {
        ModuleDefinition {
            name,
            command,
            environment,
            log_file_path,
            dependencies,
            working_dir,
        }
    }
}
