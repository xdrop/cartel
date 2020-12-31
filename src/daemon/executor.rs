use super::error::DaemonError;
use super::logs::log_file_path;
use super::module::ModuleDefinition;
use super::time::epoch_now;

use anyhow::{bail, Context, Result};
use log::{debug, error, info};
use std::collections::HashMap;
use std::ffi::OsString;
use std::fs::File;
use std::path::Path;
use std::path::PathBuf;
use std::process::{Child, Command, ExitStatus, Stdio};
use std::str::FromStr;
use std::sync::Arc;

pub struct Executor {
    module_map: HashMap<String, ModuleStatus>,
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

    child: Option<Child>,
}

impl ModuleStatus {
    pub fn empty_from(
        module_def: &Arc<ModuleDefinition>,
        log_file_path: &Path,
    ) -> ModuleStatus {
        ModuleStatus {
            module_definition: Arc::clone(&module_def),
            status: RunStatus::WAITING,
            pid: 0,
            uptime: 0,
            child: None,
            exit_time: 0,
            exit_status: None,
            log_file_path: log_file_path.as_os_str().to_os_string(),
        }
    }
}

impl PartialEq for ModuleStatus {
    fn eq(&self, other: &Self) -> bool {
        self.module_definition == other.module_definition
            && self.status == other.status
            && self.log_file_path == other.log_file_path
    }
}

impl Executor {
    pub fn new() -> Executor {
        Executor {
            module_map: HashMap::new(),
        }
    }

    fn get_log_file_path(
        module: &ModuleDefinition,
    ) -> Result<std::path::PathBuf> {
        match &module.log_file_path {
            Some(m) => PathBuf::from_str(&m)
                .with_context(|| format!("Invalid custom log path received")),
            _ => Ok(log_file_path(&module.name)),
        }
    }

    fn prepare_log_files(log_file_path: &Path) -> Result<(File, File)> {
        let stdout_file = File::create(log_file_path)
            .with_context(|| "Failed to create log file")?;
        let stderr_file = stdout_file
            .try_clone()
            .with_context(|| "Failed to create log file")?;
        Ok((stdout_file, stderr_file))
    }

    pub fn module_status_by_name(
        &self,
        name: &String,
    ) -> Option<&ModuleStatus> {
        self.module_map.get(name)
    }

    /// Returns an iterator to module statuses.
    pub fn modules(&self) -> impl Iterator<Item = &ModuleStatus> {
        self.module_map.values()
    }

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

    /// Attempt to collect any dead processes.
    ///
    /// Looks for any processes that may have exited and updates their status as
    /// well as their exit time. If the dead process moved was `RUNNING` then
    /// that indicates a process exited (or got killed). Any other status is
    /// mapped to `STOPPED` (i.e. stopped by the user).
    pub fn collect(&mut self) -> () {
        for module in self.running_modules_mut() {
            if let Some(data) = &mut module.child {
                if let Ok(Some(status)) = data.try_wait() {
                    module.exit_time = epoch_now();
                    module.exit_status = Option::from(status);
                    module.status = match module.status {
                        RunStatus::RUNNING => RunStatus::EXITED,
                        _ => RunStatus::STOPPED,
                    };

                    info!(
                        "Collecting dead process ({}) with exit-code {:#?}",
                        module.pid,
                        status.code().unwrap_or(-1)
                    );
                }
            }
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

    /// Stops a module by name.
    ///
    /// Note: This will not stop dependent modules.
    pub fn stop_module(&mut self, name: &String) -> Result<()> {
        info!("Stopping module: {}", name);
        if let Some(module) = self.module_map.get_mut(name) {
            if let Some(child) = &mut module.child {
                module.status = RunStatus::STOPPED;
                module.exit_time = epoch_now();
                // Kill child process
                child.kill().ok();
                // Wait for exit
                child.wait().ok();
            }
        }
        Ok(())
    }

    fn spawn_child(
        command: &str,
        args: &[String],
        stdout: File,
        stderr: File,
        env: &HashMap<String, String>,
        work_dir: Option<&Path>,
    ) -> Result<Child> {
        let mut cmd = Command::new(command);

        cmd.args(args)
            .stdout(Stdio::from(stdout))
            .stderr(Stdio::from(stderr))
            .envs(env);

        if let Some(path) = work_dir {
            cmd.current_dir(path);
        }

        cmd.spawn()
            .with_context(|| format!("Unable to start process '{}'", command))
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

        let log_file_pathbuf = Self::get_log_file_path(&module)?;
        let log_file_path = log_file_pathbuf.as_path();

        let module_entry = self
            .module_map
            .entry(module.name.clone())
            .or_insert_with(|| {
                ModuleStatus::empty_from(&module, log_file_path)
            });

        let (stdout_file, stderr_file) =
            Self::prepare_log_files(log_file_path)?;
        let child = Executor::spawn_child(
            &module.command[0],
            &module.command[1..],
            stdout_file,
            stderr_file,
            &module.environment,
            module.working_dir.as_ref().map(|p| p.as_path()),
        )?;

        module_entry.status = RunStatus::RUNNING;
        module_entry.pid = child.id();
        module_entry.child = Option::Some(child);
        module_entry.uptime = epoch_now();

        info!(
            "Process ({}) started, for module {}",
            module_entry.pid, module_entry.module_definition.name
        );

        Ok(())
    }
}
