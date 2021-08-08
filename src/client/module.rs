use crate::client::cmd::{shell_to_cmd, shell_to_cmd_interactive};
use crate::dependency::{
    DependencyEdge, DependencyNode, EdgeDirection, WithDependencies, WithKey,
};
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::{fmt, iter};

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
    Shell(ShellDefinition),
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
    /// A shell is a module which allows for opening a shell to some service.
    Shell,
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
            Self::Shell => write!(f, "Shell"),
        }
    }
}

/// A definition of a module for version 1 (V1) of the daemon.
#[derive(Debug, Deserialize)]
pub struct ServiceOrTaskDefinition {
    #[serde(default = "String::default")]
    pub name: String,
    /// The command used to run the service / task.
    #[serde(default = "Vec::new")]
    pub command: Vec<String>,
    /// Alternative to `command`, where a shell executes the given statement.
    pub shell: Option<String>,
    #[serde(default = "TermSignal::default")]
    /// The termination signal to use when stopping the service.
    /// Can choose between SIGKILL, SIGTERM, SIGINT on Unix systems.
    pub termination_signal: TermSignal,
    /// The environment variables to create the process with.
    #[serde(default = "HashMap::new")]
    pub environment: HashMap<String, String>,
    #[serde(default = "HashMap::new")]
    pub environment_sets: HashMap<String, HashMap<String, String>>,
    /// A custom alternate log file path.
    pub log_file_path: Option<String>,
    /// A list of dependencies of the service / task.
    #[serde(default = "Vec::new")]
    pub dependencies: Vec<String>,
    /// A list of dependencies of the service / task that must be deployed
    /// sequentially.
    #[serde(default = "Vec::new")]
    pub ordered_dependencies: Vec<String>,
    /// A list of dependencies of the service / task that must be deployed
    /// _after_ this service/task deploys.
    #[serde(default = "Vec::new")]
    pub after: Vec<String>,
    /// A list of tasks to perform after the services readiness probe has passed.
    /// If the service has no readiness probes then this equivalent to `post`.
    #[serde(default = "Vec::new")]
    pub post_up: Vec<String>,
    /// A list of tasks to perform after the service has been deployed.
    /// This will not wait for the readiness probe to complete before starting
    /// the task.
    #[serde(default = "Vec::new")]
    pub post: Vec<String>,
    /// The working directory of the service / task.
    /// Relative or absolute paths are supported.
    pub working_dir: Option<String>,
    /// A list of checks to perform.
    #[serde(default = "Vec::new")]
    pub checks: Vec<String>,
    /// Set to false to disable always waiting readiness probes to complete
    #[serde(default = "default_always_await_readiness_probe")]
    pub always_await_readiness_probe: bool,
    /// Definition of a readiness probe for the service.
    pub readiness_probe: Option<Probe>,
    /// Definition of a liveness probe for the service.
    pub liveness_probe: Option<Probe>,
    /// Duration in seconds before a task is considered as failed (currently
    /// only for tasks).
    pub timeout: Option<u64>,
    /// If enabled and a `shell` command is given, the process will be spawned
    /// in an interactive shell based on the one the client is running on.
    #[serde(default = "default_interactive_shell")]
    pub interactive_shell: bool,
}

#[derive(Debug, Deserialize)]
/// A definition of a command which spawns a shell
pub struct ShellDefinition {
    #[serde(default = "String::default")]
    pub name: String,
    /// The service this shell is for
    pub service: String,
    /// The type of the shell. Used to choose between multiple shell options for
    /// a service.
    #[serde(rename = "type", default = "String::new")]
    pub shell_type: String,
    /// The command used to open the shell.
    #[serde(default = "Vec::new")]
    pub command: Vec<String>,
    /// Alternative to `command`, where a shell executes the given statement.
    pub shell: Option<String>,
    /// The environment variables to create the process with.
    #[serde(default = "HashMap::new")]
    pub environment: HashMap<String, String>,
    /// The working directory to execute the shell command in.
    pub working_dir: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Probe {
    Exec(ExecutableProbe),
    LogLine(LogLineProbe),
    Net(NetworkProbe),
}

#[derive(Debug, Deserialize)]
pub struct ExecutableProbe {
    /// Number of retries before the probe is considered failed.
    #[serde(default = "default_probe_retries")]
    pub retries: u32,
    /// The command to execute as the probe. Exit code zero is considered
    /// healthy.
    #[serde(default = "Vec::new")]
    pub command: Vec<String>,
    /// Alternative to `command`, where a shell executes the given statement.
    pub shell: Option<String>,
    /// The working directory where the command is performed from.
    pub working_dir: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct LogLineProbe {
    /// Number of retries before the probe is considered failed.
    #[serde(default = "default_probe_retries")]
    pub retries: u32,
    /// The regex to attempt to match on a log line.
    pub line_regex: String,
}

#[derive(Debug, Deserialize)]
pub struct NetworkProbe {
    /// Number of retries before the probe is considered failed.
    #[serde(default = "default_probe_retries")]
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
    #[serde(default = "Vec::new")]
    pub command: Vec<String>,
    /// Alternative to `command`, where a shell executes the given statement.
    pub shell: Option<String>,
    /// An optional suggested fix for this check.
    pub suggested_fix: Option<SuggestedFixDefinition>,
    /// The working dir to perform the command in.
    pub working_dir: Option<String>,
    /// An detailed error message to display the user instructing how to fix the
    /// issue the check is concerned with.
    pub help: String,
}

#[derive(Debug, Deserialize)]
pub struct SuggestedFixDefinition {
    /// A message shown to the user before the choose to accept the fix.
    pub message: String,
    /// The command used to fix the check.
    #[serde(default = "Vec::new")]
    pub command: Vec<String>,
    /// Alternative to `command`, where a shell executes the given statement.
    pub shell: Option<String>,
    /// The working directory where the command is performed from.
    pub working_dir: Option<String>,
}

impl ServiceOrTaskDefinition {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        name: String,
        command: Vec<String>,
        shell: Option<String>,
        environment: HashMap<String, String>,
        environment_sets: HashMap<String, HashMap<String, String>>,
        log_file_path: Option<String>,
        dependencies: Vec<String>,
        ordered_dependencies: Vec<String>,
        after: Vec<String>,
        post_up: Vec<String>,
        post: Vec<String>,
        working_dir: Option<String>,
        checks: Vec<String>,
        termination_signal: TermSignal,
        always_await_readiness_probe: bool,
        readiness_probe: Option<Probe>,
        liveness_probe: Option<Probe>,
        timeout: Option<u64>,
        interactive_shell: bool,
    ) -> ServiceOrTaskDefinition {
        ServiceOrTaskDefinition {
            name,
            command,
            shell,
            termination_signal,
            environment,
            environment_sets,
            log_file_path,
            dependencies,
            ordered_dependencies,
            after,
            post_up,
            post,
            working_dir,
            checks,
            always_await_readiness_probe,
            readiness_probe,
            liveness_probe,
            timeout,
            interactive_shell,
        }
    }

    /// Get the execution command of this task or service.
    ///
    /// If no command was provided then the `shell` field is used to get an
    /// appropriate command line that invokes a shell.
    pub fn cmd_line(&self) -> Vec<String> {
        if self.command.is_empty() {
            if self.interactive_shell {
                shell_to_cmd_interactive(self.shell.as_ref().unwrap())
            } else {
                shell_to_cmd(self.shell.as_ref().unwrap())
            }
        } else {
            self.command.clone()
        }
    }
}

impl CheckDefinition {
    /// Get the execution command of this check.
    ///
    /// If no command was provided then the `shell` field is used to get an
    /// appropriate command line that invokes a shell.
    pub fn cmd_line(&self) -> Vec<String> {
        if self.command.is_empty() {
            shell_to_cmd(self.shell.as_ref().unwrap())
        } else {
            self.command.clone()
        }
    }
}

impl SuggestedFixDefinition {
    /// Get the execution command of this suggested fix.
    ///
    /// If no command was provided then the `shell` field is used to get an
    /// appropriate command line that invokes a shell.
    pub fn cmd_line(&self) -> Vec<String> {
        if self.command.is_empty() {
            shell_to_cmd(self.shell.as_ref().unwrap())
        } else {
            self.command.clone()
        }
    }
}

impl ExecutableProbe {
    /// Get the execution command of this probe.
    ///
    /// If no command was provided then the `shell` field is used to get an
    /// appropriate command line that invokes a shell.
    pub fn cmd_line(&self) -> Vec<String> {
        if self.command.is_empty() {
            shell_to_cmd(self.shell.as_ref().unwrap())
        } else {
            self.command.clone()
        }
    }
}

impl ShellDefinition {
    /// Get the execution command of this shell.
    ///
    /// If no command was provided then the `shell` field is used to get an
    /// appropriate command line that invokes a shell.
    pub fn cmd_line(&self) -> Vec<String> {
        if self.command.is_empty() {
            shell_to_cmd(self.shell.as_ref().unwrap())
        } else {
            self.command.clone()
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
    WaitProbe = 2,
}

impl Default for ModuleMarker {
    fn default() -> Self {
        Self::WaitProbe
    }
}

impl Eq for ModuleDefinition {}

impl WithDependencies<ModuleMarker> for ServiceOrTaskDefinition {
    fn dependencies(&self) -> Vec<DependencyEdge<ModuleMarker>> {
        self.edges()
    }

    fn is_group(&self) -> bool {
        false
    }
}

impl WithKey for ServiceOrTaskDefinition {
    fn key(&self) -> String {
        self.name.clone()
    }

    fn key_ref(&self) -> &str {
        self.name.as_str()
    }
}

impl AsRef<ModuleDefinition>
    for &DependencyNode<&ModuleDefinition, ModuleMarker>
{
    fn as_ref(&self) -> &ModuleDefinition {
        self.value
    }
}

impl AsRef<ModuleDefinition> for ModuleDefinition {
    fn as_ref(&self) -> &ModuleDefinition {
        self
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
                edge_src: self.name.clone(),
                edge_dst: key.clone(),
                is_weak: false,
                direction: EdgeDirection::To,
                marker: ModuleMarker::WaitProbe,
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
                edge_src: self.key(),
                edge_dst: key.clone(),
                is_weak: false,
                direction: EdgeDirection::To,
                marker: ModuleMarker::WaitProbe,
            })
            .chain(
                self.ordered_dependencies
                    .windows(2)
                    .map(|window| {
                        // this sets up an edge to enforce a sequential order
                        // between dependencies
                        let in_between = DependencyEdge {
                            edge_src: window[1].clone(),
                            edge_dst: window[0].clone(),
                            is_weak: false,
                            direction: EdgeDirection::To,
                            marker: ModuleMarker::WaitProbe,
                        };
                        // this sets up the edge between the main task to the
                        // dependencies
                        window
                            .iter()
                            .map(|w| DependencyEdge {
                                edge_src: self.key(),
                                edge_dst: w.clone(),
                                is_weak: false,
                                direction: EdgeDirection::To,
                                marker: ModuleMarker::WaitProbe,
                            })
                            .chain(iter::once(in_between))
                    })
                    .flatten(),
            )
            .chain(self.after.iter().map(|key| DependencyEdge {
                edge_src: self.key(),
                edge_dst: key.clone(),
                is_weak: true,
                direction: EdgeDirection::To,
                marker: ModuleMarker::WaitProbe,
            }))
            .chain(self.post_up.iter().map(|key| DependencyEdge {
                edge_src: self.key(),
                edge_dst: key.clone(),
                is_weak: false,
                direction: EdgeDirection::From,
                marker: ModuleMarker::WaitProbe,
            }))
            .chain(self.post.iter().map(|key| DependencyEdge {
                edge_src: self.key(),
                edge_dst: key.clone(),
                is_weak: false,
                direction: EdgeDirection::From,
                marker: ModuleMarker::Instant,
            }))
            .collect();

        edges
    }
}

impl WithDependencies<ModuleMarker> for ModuleDefinition {
    fn dependencies(&self) -> Vec<DependencyEdge<ModuleMarker>> {
        match &self.inner {
            InnerDefinition::Group(group) => group.edges(),
            InnerDefinition::Task(task) => task.edges(),
            InnerDefinition::Service(service) => service.edges(),
            InnerDefinition::Check(_) => panic!("Check used as dependency"),
            InnerDefinition::Shell(_) => panic!("Shell used as dependency"),
        }
    }

    fn is_group(&self) -> bool {
        self.kind == ModuleKind::Group
    }
}

impl WithKey for ModuleDefinition {
    fn key(&self) -> String {
        self.name.clone()
    }

    fn key_ref(&self) -> &str {
        self.name.as_str()
    }
}

fn default_probe_retries() -> u32 {
    5
}

fn default_always_await_readiness_probe() -> bool {
    true
}

fn default_interactive_shell() -> bool {
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

pub fn merge_env(
    base: &mut HashMap<String, String>,
    delta: &HashMap<String, String>,
) {
    for (key, val) in delta.iter() {
        base.insert(key.clone(), val.clone());
    }
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

pub fn shell_for_service<'a>(
    service_name: &str,
    shell_type: Option<&str>,
    modules: &'a [ModuleDefinition],
) -> Option<&'a ModuleDefinition> {
    modules.iter().find(|m| match m.inner {
        InnerDefinition::Shell(ref shell) => {
            shell.service == service_name
                && shell.shell_type == shell_type.unwrap_or("")
        }
        _ => false,
    })
}
