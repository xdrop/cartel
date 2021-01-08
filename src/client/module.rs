use crate::dependency::WithDependencies;
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::hash::{Hash, Hasher};

#[derive(Deserialize)]
pub struct ModuleDefinitionV1 {
    pub name: String,
    #[serde(skip_deserializing)]
    pub kind: ModuleKindV1,
    #[serde(flatten)]
    pub inner: InnerDefinitionV1,
}

#[derive(Deserialize)]
#[serde(tag = "kind")]
pub enum InnerDefinitionV1 {
    Task(ServiceOrTaskDefinitionV1),
    Service(ServiceOrTaskDefinitionV1),
    Check(CheckDefinitionV1),
    Group(GroupDefinitionV1),
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
    /// A group is a module which serves as a grouping of other modules
    /// that need to be deployed together.
    Group,
}

/// The choice of terminating signal to use when terminating the process.
///
/// Note: Only implemented for Unix based systems.
#[derive(Debug, Deserialize, PartialEq, Clone)]
pub enum TermSignal {
    /// Translates to SIGKILL on Unix based systems.
    KILL,
    /// Translates to SIGTERM on Unix based systems.
    TERM,
    /// Translates to SIGINT on Unix based systems.
    INT,
}

impl Default for TermSignal {
    fn default() -> Self {
        Self::KILL
    }
}

impl Default for ModuleKindV1 {
    fn default() -> Self {
        Self::Service
    }
}

impl fmt::Display for ModuleKindV1 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Task => write!(f, "Task"),
            Self::Service => write!(f, "Service"),
            Self::Check => write!(f, "Check"),
            Self::Group => write!(f, "Group"),
        }
    }
}

/// A definition of a module for version 1 (V1) of the daemon.
#[derive(Debug, Deserialize)]
pub struct ServiceOrTaskDefinitionV1 {
    #[serde(default = "String::default")]
    pub name: String,
    pub command: Vec<String>,
    #[serde(default = "TermSignal::default")]
    pub termination_signal: TermSignal,
    #[serde(default = "HashMap::new")]
    pub environment: HashMap<String, String>,
    pub log_file_path: Option<String>,
    #[serde(default = "Vec::new")]
    pub dependencies: Vec<String>,
    pub working_dir: Option<String>,
    #[serde(default = "Vec::new")]
    pub checks: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct GroupDefinitionV1 {
    #[serde(default = "String::default")]
    pub name: String,
    #[serde(default = "Vec::new")]
    pub dependencies: Vec<String>,
    #[serde(default = "Vec::new")]
    pub checks: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct CheckDefinitionV1 {
    #[serde(default = "String::default")]
    pub name: String,
    pub about: String,
    pub command: Vec<String>,
    pub working_dir: Option<String>,
    pub help: String,
}

impl ServiceOrTaskDefinitionV1 {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        name: String,
        command: Vec<String>,
        environment: HashMap<String, String>,
        log_file_path: Option<String>,
        dependencies: Vec<String>,
        working_dir: Option<String>,
        checks: Vec<String>,
        termination_signal: TermSignal,
    ) -> ServiceOrTaskDefinitionV1 {
        ServiceOrTaskDefinitionV1 {
            name,
            command,
            environment,
            log_file_path,
            dependencies,
            working_dir,
            checks,
            termination_signal,
        }
    }
}

impl Hash for ModuleDefinitionV1 {
    fn hash<S: Hasher>(&self, state: &mut S) {
        self.name.hash(state);
    }
}

impl PartialEq for ModuleDefinitionV1 {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}

impl Eq for ModuleDefinitionV1 {}

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

impl WithDependencies for ModuleDefinitionV1 {
    fn key(&self) -> String {
        self.name.clone()
    }

    fn key_ref(&self) -> &str {
        self.name.as_str()
    }

    fn dependencies(&self) -> &Vec<String> {
        match &self.inner {
            InnerDefinitionV1::Group(group) => &group.dependencies,
            InnerDefinitionV1::Task(task) => &task.dependencies,
            InnerDefinitionV1::Service(service) => &service.dependencies,
            InnerDefinitionV1::Check(_) => panic!("Check used as dependency"),
        }
    }
}

pub fn module_names(modules: &[ModuleDefinitionV1]) -> Vec<&str> {
    modules.iter().map(|m| m.name.as_str()).collect()
}

pub fn module_names_set(modules: &[ModuleDefinitionV1]) -> HashSet<&str> {
    modules.iter().map(|m| m.name.as_str()).collect()
}

pub fn remove_checks(
    modules: &mut Vec<ModuleDefinitionV1>,
) -> HashMap<String, CheckDefinitionV1> {
    let mut indices = vec![];
    let mut checks = HashMap::new();

    for (idx, module) in modules.iter().enumerate().rev() {
        if let InnerDefinitionV1::Check(_) = &module.inner {
            indices.push(idx);
        }
    }

    for idx in indices {
        let module = modules.swap_remove(idx);
        // This match will always be true, is there a way to remove it?
        if let InnerDefinitionV1::Check(check) = module.inner {
            checks.insert(module.name, check);
        }
    }

    checks
}

pub fn filter_services(
    modules: &[ModuleDefinitionV1],
) -> Vec<&ServiceOrTaskDefinitionV1> {
    let mut services = vec![];

    for module in modules {
        if let InnerDefinitionV1::Service(service) = &module.inner {
            services.push(service);
        }
    }

    services
}

pub fn module_by_name<'a>(
    name: &str,
    modules: &'a [ModuleDefinitionV1],
) -> Option<&'a ModuleDefinitionV1> {
    modules.iter().find(|m| m.name == name)
}
