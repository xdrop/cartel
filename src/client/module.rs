use crate::dependency::WithDependencies;
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::hash::{Hash, Hasher};

#[derive(Deserialize)]
#[serde(tag = "kind")]
pub enum BaseDefinitionV1 {
    Task(ServiceOrTaskDefinitionV1),
    Service(ServiceOrTaskDefinitionV1),
    Check(CheckDefinitionV1),
}

/// The type of the module.
#[derive(Debug, Deserialize, PartialEq, Clone)]
pub enum ModuleKindV1 {
    /// A task is a module with a limited lifetime, used to perform some
    /// temporary operation or some setup.
    Task,
    /// A service is a longer running module. It's lifetime will be managed and
    /// can be started, stopped independently.
    Service,
    /// A check is a module which defines some condition which must evaluate to
    /// true before some service can be operated.
    Check,
}

impl Default for ModuleKindV1 {
    fn default() -> Self {
        ModuleKindV1::Service
    }
}

impl fmt::Display for ModuleKindV1 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Task => write!(f, "Task"),
            Self::Service => write!(f, "Service"),
            Self::Check => write!(f, "Check"),
        }
    }
}

/// A definition of a module for version 1 (V1) of the daemon.
#[derive(Debug, Deserialize)]
pub struct ServiceOrTaskDefinitionV1 {
    #[serde(default = "ModuleKindV1::default")]
    pub kind: ModuleKindV1,
    pub name: String,
    pub command: Vec<String>,
    pub environment: HashMap<String, String>,
    pub log_file_path: Option<String>,
    pub dependencies: Vec<String>,
    pub working_dir: Option<String>,
    #[serde(default = "Vec::new")]
    pub checks: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct CheckDefinitionV1 {
    pub name: String,
    pub about: String,
    pub command: Vec<String>,
    pub working_dir: Option<String>,
    pub help: String,
}

impl ServiceOrTaskDefinitionV1 {
    pub fn new(
        kind: ModuleKindV1,
        name: String,
        command: Vec<String>,
        environment: HashMap<String, String>,
        log_file_path: Option<String>,
        dependencies: Vec<String>,
        working_dir: Option<String>,
        checks: Vec<String>,
    ) -> ServiceOrTaskDefinitionV1 {
        ServiceOrTaskDefinitionV1 {
            kind,
            name,
            command,
            environment,
            log_file_path,
            dependencies,
            working_dir,
            checks: vec![],
        }
    }
}

impl Hash for ServiceOrTaskDefinitionV1 {
    fn hash<S: Hasher>(&self, state: &mut S) {
        self.name.hash(state);
    }
}

impl PartialEq for ServiceOrTaskDefinitionV1 {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}

impl Eq for ServiceOrTaskDefinitionV1 {}

impl WithDependencies for ServiceOrTaskDefinitionV1 {
    fn key(&self) -> String {
        self.name.clone()
    }

    fn key_ref(&self) -> &str {
        self.name.as_str()
    }

    fn dependencies(&self) -> &Vec<String> {
        &self.dependencies
    }
}

pub fn module_names(modules: &Vec<ServiceOrTaskDefinitionV1>) -> Vec<&str> {
    modules.iter().map(|m| m.name.as_str()).collect()
}

pub fn module_names_set(
    modules: &Vec<ServiceOrTaskDefinitionV1>,
) -> HashSet<&str> {
    modules.iter().map(|m| m.name.as_str()).collect()
}

pub fn module_by_name<'a>(
    name: &str,
    modules: &'a Vec<ServiceOrTaskDefinitionV1>,
) -> Option<&'a ServiceOrTaskDefinitionV1> {
    modules.iter().find(|m| m.name == name)
}

pub fn checks_index(
    checks: &Vec<CheckDefinitionV1>,
) -> HashMap<&str, &CheckDefinitionV1> {
    checks.iter().map(|chk| (chk.name.as_str(), chk)).collect()
}
