use crate::daemon::env_grabber::CurrentEnvHolder;
use crate::daemon::error::DaemonError;
use crate::daemon::logs::log_file_module;
use crate::daemon::module::{ModuleDefinition, TermSignal};
use crate::daemon::monitor::{monitor_key, MonitorType};
use crate::daemon::planner::{Monitor, MonitorHandle};
use crate::daemon::time::epoch_now;
use crate::process::{CommandExt, Process};

use crate::command_builder::CommandBuilder;
use anyhow::{Context, Result};
use log::info;
use std::borrow::Cow;
use std::collections::HashMap;
use std::ffi::OsString;
use std::fs::File;
use std::path::Path;
use std::process::ExitStatus;
use std::sync::Arc;

pub struct Executor {
    module_map: HashMap<String, ModuleStatus>,
    cfg: Arc<ExecutorConfig>,
    monitor_handle: MonitorHandle,
    env_holder: Arc<CurrentEnvHolder>,
}

pub struct ExecutorConfig {
    pub use_env_grabber_env: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum RunStatus {
    RUNNING,
    WAITING,
    STOPPED,
    EXITED,
}

#[derive(Debug)]
pub struct ModuleStatus {
    pub module_definition: Arc<ModuleDefinition>,
    pub status: RunStatus,
    pub pid: u32,
    pub uptime: u64,
    pub exit_time: u64,
    pub exit_status: Option<ExitStatus>,
    pub log_file_path: OsString,
    pub monitor_key: Option<String>,

    child: Option<Process>,
}

impl ModuleStatus {
    pub fn empty_from(
        module_def: &Arc<ModuleDefinition>,
        log_file_path: &Path,
    ) -> ModuleStatus {
        ModuleStatus {
            module_definition: Arc::clone(module_def),
            status: RunStatus::WAITING,
            pid: 0,
            uptime: 0,
            child: None,
            exit_time: 0,
            exit_status: None,
            monitor_key: None,
            log_file_path: log_file_path.as_os_str().to_os_string(),
        }
    }
}

impl Executor {
    pub fn new(
        monitor_handle: MonitorHandle,
        env_holder: Arc<CurrentEnvHolder>,
        cfg: Arc<ExecutorConfig>,
    ) -> Executor {
        Executor {
            module_map: HashMap::new(),
            monitor_handle,
            env_holder,
            cfg,
        }
    }

    /// Returns the status of module by name.
    pub fn module_status_by_name(&self, name: &str) -> Option<&ModuleStatus> {
        self.module_map.get(name)
    }

    /// Returns module statuses by names (or None if they don't exist).
    pub fn module_statuses_by_names(
        &self,
        names: &[&str],
    ) -> Vec<Option<&ModuleStatus>> {
        names
            .iter()
            .map(|name| self.module_map.get(*name))
            .collect()
    }

    /// Returns an iterator to module statuses.
    pub fn modules(&self) -> impl Iterator<Item = &ModuleStatus> {
        self.module_map.values()
    }

    /// Attempt to collect any dead processes.
    ///
    /// Looks for any processes that may have exited and updates their status as
    /// well as their exit time. If the dead process moved was `RUNNING` then
    /// that indicates a process exited (or got killed). Any other status is
    /// mapped to `STOPPED` (i.e. stopped by the user).
    pub fn collect(&mut self) {
        let mut expired_probes = vec![];

        for module in self.running_modules_mut() {
            if let Some(process) = &mut module.child {
                if let Ok(Some(status)) = process.try_wait() {
                    module.exit_time = epoch_now();
                    module.exit_status = Option::from(status);
                    module.status = match module.status {
                        RunStatus::RUNNING => RunStatus::EXITED,
                        _ => RunStatus::STOPPED,
                    };
                    if let Some(handle) = module.monitor_key.take() {
                        expired_probes.push(handle);
                    }
                    info!(
                        "Collecting dead process ({}) with exit-code {:#?}",
                        module.pid,
                        status.code().unwrap_or(-1)
                    );
                }
            }
        }

        // Remove liveness probes
        for handle in expired_probes.into_iter() {
            self.monitor_handle
                .remove_monitor(handle, MonitorType::Liveness);
        }
    }

    /// Redeploys a module with a newer module definition.
    pub fn redeploy_module(
        &mut self,
        module: Arc<ModuleDefinition>,
    ) -> Result<()> {
        info!("Redeploying module: {}", module.name);
        self.stop_module(&module.name)?;
        self.run_module(module)
    }

    /// Restarts a module (re-using the same module definition).
    pub fn restart_module(&mut self, module_name: &str) -> Result<()> {
        info!("Restarting module: {}", module_name);
        let existing = {
            let module = self.module_status_by_name(module_name);
            Arc::clone(
                &module
                    .ok_or_else(|| {
                        DaemonError::NotFound(module_name.to_string())
                    })?
                    .module_definition,
            )
        };
        self.stop_module(module_name)?;
        self.run_module(existing)
    }

    /// Stops a module by name.
    ///
    /// Note: This will not stop dependent modules.
    pub fn stop_module(&mut self, name: &str) -> Result<()> {
        info!("Stopping module: {}", name);
        match self.module_map.get_mut(name) {
            Some(module) => {
                if let Some(process) = &mut module.child {
                    // Bail if already stopped
                    if module.status != RunStatus::RUNNING {
                        return Ok(());
                    }

                    module.status = RunStatus::STOPPED;
                    module.exit_time = epoch_now();

                    // Remove monitor tracking its liveness
                    if let Some(monitor_key) = &module.monitor_key {
                        self.monitor_handle.remove_monitor(
                            monitor_key.clone(),
                            MonitorType::Liveness,
                        );
                    }

                    let module_name = module.module_definition.name.clone();

                    // Signal child process to die
                    match module.module_definition.termination_signal {
                        TermSignal::KILL => process.kill(),
                        TermSignal::TERM => process.terminate(),
                        TermSignal::INT => process.interrupt(),
                    }
                    .with_context(|| {
                        format!(
                            "Failed to signal process {} to stop",
                            module_name
                        )
                    })?;

                    process.wait()?;
                }
                Ok(())
            }
            None => Err(DaemonError::NotRunning(name.to_string()).into()),
        }
    }

    /// Executes a service module, and registers its state.
    ///
    /// The service is expected to be a long-running process and is run as a
    /// child of the daemon.
    ///
    /// # Arguments
    /// * `module` - The module definition of the service
    pub fn run_module(&mut self, module: Arc<ModuleDefinition>) -> Result<()> {
        info!("Executing module: {}", module.name);

        let log_file_pathbuf = log_file_module(&module)?;
        let log_file_path = log_file_pathbuf.as_path();
        let liveness_probe = self.maybe_create_liveness_probe(&module);
        let environment_variables = Self::environment_variables(
            &module,
            &self.env_holder,
            self.cfg.use_env_grabber_env,
        );

        let module_entry = self
            .module_map
            .entry(module.name.clone())
            .or_insert_with(|| {
                ModuleStatus::empty_from(&module, log_file_path)
            });

        let (stdout_file, stderr_file) =
            Self::prepare_log_files(log_file_path)?;

        let mut cmd = CommandBuilder::new(&module.command);
        cmd.env(&environment_variables)
            .stdout_file(stdout_file)
            .stderr_file(stderr_file)
            .work_dir(module.working_dir.as_deref());

        let child = cmd.build().group_spawn().with_context(|| {
            format!("Failed to run service '{}'", module.name)
        })?;

        module_entry.status = RunStatus::RUNNING;
        module_entry.pid = child.id();
        module_entry.child = Some(Process::groupped(child));
        module_entry.uptime = epoch_now();
        module_entry.module_definition = Arc::clone(&module);
        module_entry.monitor_key = liveness_probe;

        info!(
            "Process ({}) started, for module {}",
            module_entry.pid, module_entry.module_definition.name
        );

        Ok(())
    }

    /// Perform cleanup by attempting to kill all running child processes.
    pub fn cleanup(&mut self) -> Result<()> {
        let module_names: Vec<String> = self
            .running_modules()
            .map(|m| m.module_definition.name.clone())
            .collect();
        for module in module_names {
            self.stop_module(module.as_str())?;
        }
        Ok(())
    }
}

impl Executor {
    fn running_modules(&self) -> impl Iterator<Item = &ModuleStatus> {
        self.module_map
            .values()
            .into_iter()
            .filter(|m| m.status == RunStatus::RUNNING)
    }

    fn running_modules_mut(
        &mut self,
    ) -> impl Iterator<Item = &mut ModuleStatus> {
        self.module_map
            .values_mut()
            .filter(|m| m.status == RunStatus::RUNNING)
    }

    fn merge_envs(
        mut base_env: HashMap<String, String>,
        env: &HashMap<String, String>,
    ) -> HashMap<String, String> {
        for (key, val) in env.iter() {
            base_env.insert(key.clone(), val.clone());
        }
        base_env
    }

    fn environment_variables<'a>(
        module: &'a ModuleDefinition,
        env_holder: &CurrentEnvHolder,
        use_env_grabber_env: bool,
    ) -> Cow<'a, HashMap<String, String>> {
        if use_env_grabber_env {
            let merged_environment =
                Self::merge_envs(env_holder.read(), &module.environment);
            Cow::Owned(merged_environment)
        } else {
            Cow::Borrowed(&module.environment)
        }
    }

    fn maybe_create_liveness_probe(
        &self,
        module_def: &ModuleDefinition,
    ) -> Option<String> {
        module_def.liveness_probe.as_ref().map(|probe| {
            self.create_liveness_probe(&module_def.name, probe.clone())
        })
    }

    fn create_liveness_probe(
        &self,
        module_name: &str,
        liveness_probe: Monitor,
    ) -> String {
        let monitor_key = monitor_key(module_name, &MonitorType::Liveness);
        self.monitor_handle.new_monitor(
            monitor_key.clone(),
            liveness_probe,
            MonitorType::Liveness,
        );
        monitor_key
    }

    pub(super) fn prepare_log_files(
        log_file_path: &Path,
    ) -> Result<(File, File)> {
        let stdout_file = File::create(log_file_path)
            .with_context(|| "Failed to create log file")?;
        let stderr_file = stdout_file
            .try_clone()
            .with_context(|| "Failed to create log file")?;
        Ok((stdout_file, stderr_file))
    }
}

pub mod task_executor {
    use super::Executor;
    use crate::command_builder::CommandBuilder;
    use crate::daemon::env_grabber::CurrentEnvHolder;
    use crate::daemon::error::DaemonError;
    use crate::daemon::executor::ExecutorConfig;
    use crate::daemon::logs::log_file_module;
    use crate::daemon::module::{ModuleDefinition, ModuleKind};
    use anyhow::{Context, Result};
    use std::process::ExitStatus;
    use std::sync::Arc;

    /// Executes a task and waits for it until it is finished.
    ///
    /// The task will block the current thread, and report its exit status on
    /// completion. If the task exits with any code other than zero then an
    /// Error is thrown.
    pub fn execute_task(
        task_definition: &ModuleDefinition,
        cfg: &ExecutorConfig,
        env_holder: Arc<CurrentEnvHolder>,
    ) -> Result<ExitStatus> {
        assert!(task_definition.kind == ModuleKind::Task);
        let log_file_pathbuf = log_file_module(task_definition)?;
        let log_file_path = log_file_pathbuf.as_path();
        let environment_vars = Executor::environment_variables(
            task_definition,
            &env_holder,
            cfg.use_env_grabber_env,
        );

        let (stdout_file, stderr_file) =
            Executor::prepare_log_files(log_file_path)?;

        let mut cmd = CommandBuilder::new(&task_definition.command);
        cmd.env(&environment_vars)
            .stdout_file(stdout_file)
            .stderr_file(stderr_file)
            .work_dir(task_definition.working_dir.as_deref());

        let exit_status = cmd
            .build()
            .spawn()
            .with_context(|| {
                format!(
                    "Failed to start task {}",
                    &task_definition.command.join(" ")
                )
            })?
            .wait()
            .with_context(|| {
                format!("Task {} failed to execute", task_definition.name)
            })?;

        if !exit_status.success() {
            return Err(DaemonError::TaskFailed {
                task_name: task_definition.name.clone(),
                code: exit_status.code().unwrap_or(-1),
                log_file: log_file_path.as_os_str().to_os_string(),
            }
            .into());
        }
        Ok(exit_status)
    }
}
