use super::error::DaemonError;
use super::executor::{task_executor, Executor, ModuleStatus, RunStatus};
use super::module::ModuleDefinition;
pub use crate::daemon::monitor::{Monitor, MonitorHandle, MonitorStatus};
use anyhow::Result;
use std::collections::{HashMap, HashSet};
use std::ffi::OsString;
use std::iter::FromIterator;
use std::sync::{Arc, Mutex, MutexGuard};

pub struct Planner {
    executor: Mutex<Executor>,
    monitor_handle: MonitorHandle,
}

pub struct PsStatus {
    pub name: String,
    pub pid: u32,
    pub status: RunStatus,
    pub exit_code: Option<i32>,
    pub time_since_status: u64,
}

impl Planner {
    pub fn new(monitor_handle: MonitorHandle) -> Planner {
        Planner {
            executor: Mutex::new(Executor::new()),
            monitor_handle,
        }
    }

    /// Deploys a module (or does nothing if the module is already in the
    /// correct state).
    ///
    /// Returns true if the module was deployed, and false if the module was
    /// already in the correct state.
    ///
    /// # Arguments
    /// * `module_def` - The module definition of the module
    pub fn deploy(&self, module_def: ModuleDefinition) -> Result<bool> {
        let mut executor = self.executor();
        let existing = executor.module_status_by_name(&module_def.name);

        match existing {
            Some(module_status) => {
                if Self::should_restart(&module_def, module_status) {
                    executor.redeploy_module(Arc::new(module_def))?;
                    Ok(true)
                } else {
                    Ok(false)
                }
            }
            None => executor.run_module(Arc::new(module_def)).map(|_| true),
        }
    }

    /// Deploys one or more modules (modules already in the correct state do not
    /// get affected).
    ///
    /// Returns a map containing whether each the module was deployed, and false
    /// if the module was already in the correct state..
    ///
    /// # Arguments
    /// * `module_def` - The module definition of the module
    pub fn deploy_many(
        &self,
        module_defs: Vec<ModuleDefinition>,
        selection: &[String],
    ) -> Result<HashMap<String, bool>> {
        Self::deployment_set(module_defs, selection)?
            .map(|module_def| {
                let name = module_def.name.clone();
                let result = self.deploy(module_def)?;
                Ok((name, result))
            })
            .collect()
    }

    pub fn deploy_task(task_definition: &ModuleDefinition) -> Result<i32> {
        task_executor::execute_task(task_definition)
            .map(|exit_status| exit_status.code().unwrap_or(-1))
    }

    /// Restarts an existing module.
    ///
    /// The module could either be running, stopped or exited and the module
    /// definition of the last attempted deploy will be used.
    pub fn restart_module(&self, mod_name: &str) -> Result<()> {
        self.executor().restart_module(mod_name)
    }

    /// Stops a running module.
    pub fn stop_module(&self, mod_name: &str) -> Result<()> {
        self.executor().stop_module(mod_name)
    }

    /// Returns the log path of a running module.
    pub fn log_path(&self, mod_name: &str) -> Result<OsString> {
        let executor = self.executor();
        executor
            .module_status_by_name(mod_name)
            .ok_or_else(|| DaemonError::NotFound(mod_name.to_string()).into())
            .map(|m| m.log_file_path.clone())
    }

    /// Returns a summarized version of each modules status.
    pub fn module_status(&self) -> Vec<PsStatus> {
        self.executor()
            .modules()
            .map(|m| PsStatus {
                name: m.module_definition.name.clone(),
                pid: m.pid,
                status: m.status.clone(),
                exit_code: m.exit_status.and_then(|e| e.code()),
                time_since_status: match m.status {
                    RunStatus::RUNNING => m.uptime,
                    RunStatus::STOPPED => m.exit_time,
                    RunStatus::EXITED => m.exit_time,
                    RunStatus::WAITING => 0,
                },
            })
            .collect()
    }

    /// Collects all dead processes (and updates their status).
    ///
    /// Typically called on SIGCHLD, or via a periodic poll on systems that
    /// don't support it.
    pub fn collect_dead(&self) {
        self.executor().collect()
    }

    pub fn cleanup(&self) -> Result<()> {
        self.executor().cleanup()
    }

    pub fn stop_all(&self) -> Result<()> {
        // Currently uses cleanup, but having this as a separate function since
        // it may change in the future.
        self.executor().cleanup()
    }

    pub fn create_monitor(&self, name: String, monitor: Monitor) -> String {
        let monitor_key = format!("{}-{}", name, uuid::Uuid::new_v4());
        self.monitor_handle
            .new_monitor(monitor_key.clone(), monitor);
        monitor_key
    }

    pub fn monitor_status(&self, monitor_name: &str) -> Option<MonitorStatus> {
        self.monitor_handle.monitor_status(monitor_name)
    }
}

impl Planner {
    fn executor(&self) -> MutexGuard<Executor> {
        self.executor.lock().expect("Poisoned lock")
    }

    fn should_restart(
        module_def: &ModuleDefinition,
        module_status: &ModuleStatus,
    ) -> bool {
        if module_status.status != RunStatus::RUNNING {
            return true;
        }
        let current = module_status.module_definition.as_ref();
        current.command != module_def.command
            || current.environment != module_def.environment
            || current.log_file_path != module_def.log_file_path
            || current.working_dir != module_def.working_dir
    }

    fn deployment_set(
        module_defs: Vec<ModuleDefinition>,
        selected: &[String],
    ) -> Result<impl Iterator<Item = ModuleDefinition>> {
        let module_set: HashSet<String> = HashSet::from_iter(
            module_defs
                .iter()
                .map(|m| m.name.clone())
                .collect::<Vec<String>>(),
        );

        let selection_set: HashSet<String> = selected.iter().cloned().collect();

        if !selection_set.is_subset(&module_set) {
            return Err(DaemonError::SubsetNotFound.into());
        }

        Ok(module_defs
            .into_iter()
            .filter(move |m| selection_set.contains(&m.name)))
    }
}
