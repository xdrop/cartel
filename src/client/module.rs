use crate::dependency::{DependencyEdge, EdgeDirection, WithDependencies};
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::hash::{Hash, Hasher};

#[derive(Deserialize, Debug)]
pub struct ModuleDefinition {
    pub name: String,
    #[serde(skip_deserializing)]
    pub kind: ModuleKind,
    #[serde(flatten)]
    pub inner: InnerDefinition,
}

#[derive(Deserialize, Debug)]
#[serde(tag = "kind")]
pub enum InnerDefinition {
    Task(ServiceOrTaskDefinition),
    Service(ServiceOrTaskDefinition),
    Check(CheckDefinition),
    Group(GroupDefinition),
}

/// The type of the module.
#[derive(Debug, Deserialize, PartialEq, Clone)]
pub enum ModuleKind {
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

impl Default for ModuleKind {
    fn default() -> Self {
        Self::Service
    }
}

impl fmt::Display for ModuleKind {
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
pub struct ServiceOrTaskDefinition {
    #[serde(default = "String::default")]
    pub name: String,
    /// The command used to run the service / task.
    pub command: Vec<String>,
    #[serde(default = "TermSignal::default")]
    /// The termination signal to use when stopping the service.
    /// Can choose between SIGKILL, SIGTERM, SIGINT on Unix systems.
    pub termination_signal: TermSignal,
    /// The environment variables to create the process with.
    #[serde(default = "HashMap::new")]
    pub environment: HashMap<String, String>,
    /// A custom alternate log file path.
    pub log_file_path: Option<String>,
    #[serde(default = "Vec::new")]
    /// A list of dependencies of the service / task.
    pub dependencies: Vec<String>,
    /// A list of tasks to perform after the services healthcheck has passed.
    /// If the service has no healthcheck then this equivalent to `post`.
    #[serde(default = "Vec::new")]
    pub post_up: Vec<String>,
    /// A list of tasks to perform after the service has been deployed.
    /// This will not wait for the healthcheck to complete before starting
    /// the task.
    #[serde(default = "Vec::new")]
    pub post: Vec<String>,
    /// The working directory of the service / task.
    /// Relative or absolute paths are supported.
    pub working_dir: Option<String>,
    /// A list of checks to perform.
    #[serde(default = "Vec::new")]
    pub checks: Vec<String>,
    /// Set to true for healthcheck to always be waited for completion.
    #[serde(default = "default_always_wait_healthcheck")]
    pub always_wait_healthcheck: bool,
    /// Definition of a healthcheck for the service.
    pub healthcheck: Option<Healthcheck>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Healthcheck {
    Exec(ExecutableHealthcheck),
    LogLine(LogLineHealthcheck),
    Net(NetworkHealthcheck),
}

#[derive(Debug, Deserialize)]
pub struct ExecutableHealthcheck {
    /// Number of retries before the healthcheck is considered failed.
    #[serde(default = "default_healthcheck_retries")]
    pub retries: u32,
    /// The command to execute as the healthcheck. Exit code zero is considered
    /// healthy.
    pub command: Vec<String>,
    /// The working directory where the command is performed from.
    pub working_dir: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct LogLineHealthcheck {
    /// Number of retries before the healthcheck is considered failed.
    #[serde(default = "default_healthcheck_retries")]
    pub retries: u32,
    /// The regex to attempt to match on a log line.
    pub line_regex: String,
}

#[derive(Debug, Deserialize)]
pub struct NetworkHealthcheck {
    /// Number of retries before the healthcheck is considered failed.
    #[serde(default = "default_healthcheck_retries")]
    pub retries: u32,
    /// The host to try and connect to.
    pub host: String,
    /// The port to try and connect to.
    pub port: u16,
}

#[derive(Debug, Deserialize)]
pub struct GroupDefinition {
    #[serde(default = "String::default")]
    pub name: String,
    /// A list of dependencies of the group.
    #[serde(default = "Vec::new")]
    pub dependencies: Vec<String>,
    /// A list of checks to perform.
    #[serde(default = "Vec::new")]
    pub checks: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct CheckDefinition {
    #[serde(default = "String::default")]
    pub name: String,
    /// A short description of the check checks for.
    pub about: String,
    /// The command used to perform the check. The command should exit with code
    /// zero to be considered a pass.
    pub command: Vec<String>,
    /// The working dir to perform the command in.
    pub working_dir: Option<String>,
    /// An detailed error message to display the user instructing how to fix the
    /// issue the check is concerned with.
    pub help: String,
}

impl ServiceOrTaskDefinition {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        name: String,
        command: Vec<String>,
        environment: HashMap<String, String>,
        log_file_path: Option<String>,
        dependencies: Vec<String>,
        post_up: Vec<String>,
        post: Vec<String>,
        working_dir: Option<String>,
        checks: Vec<String>,
        termination_signal: TermSignal,
        always_wait_healthcheck: bool,
        healthcheck: Option<Healthcheck>,
    ) -> ServiceOrTaskDefinition {
        ServiceOrTaskDefinition {
            name,
            command,
            environment,
            log_file_path,
            dependencies,
            post_up,
            post,
            working_dir,
            checks,
            termination_signal,
            always_wait_healthcheck,
            healthcheck,
        }
    }
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

#[derive(Copy, Clone, PartialEq, PartialOrd, Debug)]
pub enum ModuleMarker {
    Instant = 1,
    WaitHealthcheck = 2,
}

impl Default for ModuleMarker {
    fn default() -> Self {
        Self::WaitHealthcheck
    }
}

impl Eq for ModuleDefinition {}

impl WithDependencies<ModuleMarker> for ServiceOrTaskDefinition {
    fn key(&self) -> String {
        self.name.clone()
    }

    fn key_ref(&self) -> &str {
        self.name.as_str()
    }

    fn dependencies(&self) -> Vec<DependencyEdge<ModuleMarker>> {
        self.edges()
    }
}

trait EdgeList {
    fn edges(&self) -> Vec<DependencyEdge<ModuleMarker>>;
}

impl EdgeList for GroupDefinition {
    fn edges(&self) -> Vec<DependencyEdge<ModuleMarker>> {
        self.dependencies
            .iter()
            .map(|key| DependencyEdge {
                edge_ptr: key.clone(),
                direction: EdgeDirection::To,
                marker: ModuleMarker::WaitHealthcheck,
            })
            .collect()
    }
}

impl EdgeList for ServiceOrTaskDefinition {
    fn edges(&self) -> Vec<DependencyEdge<ModuleMarker>> {
        let edges: Vec<DependencyEdge<ModuleMarker>> = self
            .dependencies
            .iter()
            .map(|key| DependencyEdge {
                edge_ptr: key.clone(),
                direction: EdgeDirection::To,
                marker: ModuleMarker::WaitHealthcheck,
            })
            .chain(self.post_up.iter().map(|key| DependencyEdge {
                edge_ptr: key.clone(),
                direction: EdgeDirection::From,
                marker: ModuleMarker::WaitHealthcheck,
            }))
            .chain(self.post.iter().map(|key| DependencyEdge {
                edge_ptr: key.clone(),
                direction: EdgeDirection::From,
                marker: ModuleMarker::Instant,
            }))
            .collect();

        edges
    }
}

impl WithDependencies<ModuleMarker> for ModuleDefinition {
    fn key(&self) -> String {
        self.name.clone()
    }

    fn key_ref(&self) -> &str {
        self.name.as_str()
    }

    fn dependencies(&self) -> Vec<DependencyEdge<ModuleMarker>> {
        match &self.inner {
            InnerDefinition::Group(group) => group.edges(),
            InnerDefinition::Task(task) => task.edges(),
            InnerDefinition::Service(service) => service.edges(),
            InnerDefinition::Check(_) => panic!("Check used as dependency"),
        }
    }
}

fn default_healthcheck_retries() -> u32 {
    5
}

fn default_always_wait_healthcheck() -> bool {
    false
}

pub fn module_names(modules: &[ModuleDefinition]) -> Vec<&str> {
    modules.iter().map(|m| m.name.as_str()).collect()
}

pub fn module_names_set(modules: &[ModuleDefinition]) -> HashSet<&str> {
    modules.iter().map(|m| m.name.as_str()).collect()
}

pub fn remove_checks(
    modules: &mut Vec<ModuleDefinition>,
) -> HashMap<String, CheckDefinition> {
    let mut indices = vec![];
    let mut checks = HashMap::new();

    for (idx, module) in modules.iter().enumerate().rev() {
        if let InnerDefinition::Check(_) = &module.inner {
            indices.push(idx);
        }
    }

    for idx in indices {
        let module = modules.swap_remove(idx);
        // This match will always be true, is there a way to remove it?
        if let InnerDefinition::Check(check) = module.inner {
            checks.insert(module.name, check);
        }
    }

    checks
}

pub fn filter_services(
    modules: &[ModuleDefinition],
) -> Vec<&ServiceOrTaskDefinition> {
    let mut services = vec![];

    for module in modules {
        if let InnerDefinition::Service(service) = &module.inner {
            services.push(service);
        }
    }

    services
}

pub fn module_by_name<'a>(
    name: &str,
    modules: &'a [ModuleDefinition],
) -> Option<&'a ModuleDefinition> {
    modules.iter().find(|m| m.name == name)
}
