use crate::client::emoji::TEXTBOOK;
use crate::client::module::{
    CheckDefinition, GroupDefinition, InnerDefinition, ModuleDefinition,
    ModuleMarker, ServiceOrTaskDefinition,
};

use crate::client::process::run_check;
use crate::client::progress::{SpinnerOptions, WaitResult, WaitUntil};
use crate::client::request;
use crate::client::{cli::ClientConfig, commands::DeployOptions};
use crate::daemon::api::ApiProbeStatus;
use anyhow::{anyhow, bail, Result};
use crossbeam_queue::ArrayQueue;
use indicatif::MultiProgress;
use indicatif::ProgressBar;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

pub struct Deployer {
    multiprogress: Arc<MultiProgress>,
    queue: Arc<ArrayQueue<usize>>,
}

pub struct ModuleToDeploy<'a> {
    pub definition: &'a ModuleDefinition,
    pub marker: Option<ModuleMarker>,
}

impl Deployer {
    pub fn new(
        multiprogress: Arc<MultiProgress>,
        queue: Arc<ArrayQueue<usize>>,
    ) -> Self {
        Self {
            multiprogress,
            queue,
        }
    }

    pub fn do_work(
        &self,
        modules: &[ModuleToDeploy],
        cfg: &ClientConfig,
        deploy_opts: &DeployOptions,
    ) -> Result<()> {
        while !self.queue.is_empty() {
            if let Some(idx) = self.queue.pop() {
                let module_to_deploy = &modules[idx];
                self.deploy_module(module_to_deploy, cfg, deploy_opts)?;
            }
        }
        Ok(())
    }

    fn deploy_and_maybe_wait_service(
        &self,
        service: &ServiceOrTaskDefinition,
        marker: Option<ModuleMarker>,
        cfg: &ClientConfig,
        deploy_opts: &DeployOptions,
    ) -> Result<()> {
        let monitor_handle = self.deploy_service(service, cfg, deploy_opts)?;
        let node_marked = marker == Some(ModuleMarker::WaitProbe);

        if let Some(handle) = monitor_handle {
            if (node_marked
                || service.always_await_readiness_probe
                || deploy_opts.wait)
                && !deploy_opts.skip_readiness_checks
            {
                self.wait_until_healthy(
                    service.name.as_str(),
                    handle.as_str(),
                    cfg,
                )?;
            }
        }
        Ok(())
    }

    fn deploy_module(
        &self,
        module: &ModuleToDeploy,
        cfg: &ClientConfig,
        deploy_opts: &DeployOptions,
    ) -> Result<()> {
        match module.definition.inner {
            InnerDefinition::Task(ref task) => self.deploy_task(task, cfg),
            InnerDefinition::Service(ref service) => self
                .deploy_and_maybe_wait_service(
                    service,
                    module.marker,
                    cfg,
                    deploy_opts,
                ),
            InnerDefinition::Group(ref group) => {
                self.deploy_group(group);
                Ok(())
            }
            InnerDefinition::Check(_) => Ok(()),
            InnerDefinition::Shell(_) => Ok(()),
        }?;
        Ok(())
    }

    fn deploy_service(
        &self,
        module: &ServiceOrTaskDefinition,
        cfg: &ClientConfig,
        deploy_opts: &DeployOptions,
    ) -> Result<Option<String>> {
        let message = format!("Deploying {}", cbold!(&module.name));
        let spin_opt = SpinnerOptions::new(message).clear_on_finish(false);

        let pb = self.multiprogress.add(ProgressBar::new(std::u64::MAX));
        let wu = WaitUntil::new_multi(&spin_opt, pb);
        let deploy_result = wu.spin_until_status(|| {
            let result = request::deploy_module(
                module,
                deploy_opts.force_deploy,
                &cfg.daemon_url,
            )?;

            let deploy_status = if result.deployed {
                csuccess!("(Deployed)")
            } else {
                cdim!("(Already deployed)")
            };
            Ok(WaitResult::from(result, deploy_status.to_string()))
        })?;

        let monitor_handle = deploy_result.monitor;
        Ok(monitor_handle)
    }

    fn wait_until_healthy(
        &self,
        module_name: &str,
        monitor_handle: &str,
        cfg: &ClientConfig,
    ) -> Result<()> {
        let message = format!("Waiting {} to be healthy", cbold!(module_name));
        let spin_opt = SpinnerOptions::new(message).clear_on_finish(false);
        let pb = self.multiprogress.add(ProgressBar::new(std::u64::MAX));
        let wu = WaitUntil::new_multi(&spin_opt, pb);

        wu.spin_until_status(|| loop {
            let status = csuccess!("(Done)").to_string();
            match request::poll_health(monitor_handle, &cfg.daemon_url)?
                .probe_status
            {
                Some(ApiProbeStatus::Successful) => {
                    break Ok(WaitResult::from((), status))
                }
                Some(ApiProbeStatus::RetriesExceeded) => {
                    bail!(
                        "The service did not complete its readiness probe checks in time.\n\
                        Check the logs for more details."
                    )
                }
                Some(ApiProbeStatus::Error) => {
                    bail!(
                        "An error occured while waiting for the service \
                        readiness probe to complete.\nThis is usually a mistake in \
                        the probe configuration, ensure the command or \
                        condition is correct."
                    )
                }
                _ => {
                    std::thread::sleep(Duration::from_secs(2));
                }
        }
    })?;

        Ok(())
    }

    fn deploy_task(
        &self,
        module: &ServiceOrTaskDefinition,
        cfg: &ClientConfig,
    ) -> Result<()> {
        let message = format!("Running task {}", cbold!(&module.name));
        let spin_opt = SpinnerOptions::new(message).clear_on_finish(false);

        let pb = self.multiprogress.add(ProgressBar::new(std::u64::MAX));
        let wu = WaitUntil::new_multi(&spin_opt, pb);
        wu.spin_until_status(|| {
            let result = request::deploy_task(module, &cfg.daemon_url)?;
            let status = csuccess!("(Done)").to_string();
            Ok(WaitResult::from(result, status))
        })?;

        Ok(())
    }

    fn deploy_group(&self, module: &GroupDefinition) {
        let message = format!("Group {}", cbold!(&module.name));
        tiprint!(
            10, // indent level
            "{} {}",
            message,
            csuccess!("(Done)")
        );
    }

    pub fn perform_check(check_def: &CheckDefinition) -> Result<()> {
        let message =
            format!("Check {} ({})", cbold!(&check_def.about), check_def.name);
        let spin_opt = SpinnerOptions::new(message).clear_on_finish(false);
        let wu = WaitUntil::new(&spin_opt);

        let check_result = wu.spin_until_status(|| {
            let check_result = run_check(check_def)?;
            let status = if check_result.success() {
                csuccess!("(OK)")
            } else {
                cfail!("(FAIL)")
            };
            Ok(WaitResult::from(check_result, status.to_string()))
        })?;

        if !check_result.success() {
            bail!(
                "The {} check has failed\n\
            {}: {}",
                cbold!(&check_def.about),
                cbold!("Message"),
                check_def.help
            )
        }
        Ok(())
    }

    pub fn run_checks<T: AsRef<ModuleDefinition>>(
        checks_map: HashMap<String, CheckDefinition>,
        modules: &[T],
        deploy_opts: &DeployOptions,
    ) -> Result<()> {
        if deploy_opts.skip_checks {
            let msg = format!("Running checks... {}", cdim!("(Skip)"));
            tprintstep!(msg, 3, 5, TEXTBOOK);
        } else {
            tprintstep!("Running checks...", 3, 5, TEXTBOOK);
            for m in modules {
                let checks = match &m.as_ref().inner {
                    InnerDefinition::Group(grp) => grp.checks.as_slice(),
                    InnerDefinition::Service(srvc) => srvc.checks.as_slice(),
                    InnerDefinition::Task(tsk) => tsk.checks.as_slice(),
                    _ => &[],
                };

                for check in checks {
                    let check = checks_map.get(check).ok_or_else(|| {
                        anyhow!("Check '{}' not defined", check)
                    })?;

                    Self::perform_check(check)?;
                }
            }
        }
        Ok(())
    }
}
