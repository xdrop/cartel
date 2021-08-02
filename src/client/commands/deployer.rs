use crate::client::cli::ClientConfig;
use crate::client::commands::DeployOptions;
use crate::client::emoji::{HOUR_GLASS, SUCCESS, YELLOW_NOTEBOOK};
use crate::client::module::{
    CheckDefinition, GroupDefinition, InnerDefinition, ModuleDefinition,
    ModuleMarker, ServiceOrTaskDefinition, SuggestedFixDefinition,
};
use crate::client::process::{apply_suggested_fix, run_check};
use crate::client::progress::{
    SpinnerOptions, WaitResult, WaitSpin, WaitUntil,
};
use crate::client::request;
use crate::client::request::get_plan;
use crate::daemon::api::{
    ApiGetPlanResponse, ApiPlannedAction, ApiProbeStatus,
};
use crate::dependency::DependencyNode;
use anyhow::{anyhow, bail, Result};
use crossbeam_queue::ArrayQueue;
use indicatif::{MultiProgress, ProgressBar};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;
use text_io::read;

pub struct Deployer {
    multiprogress: Arc<MultiProgress>,
    queue: Arc<ArrayQueue<usize>>,
    deployment_plan: Option<Arc<ModuleDeploymentPlan>>,
}

pub struct ModuleDeploymentPlan {
    pub should_deploy: HashMap<String, bool>,
}

pub struct ModuleToDeploy<'a> {
    pub definition: &'a ModuleDefinition,
    pub marker: Option<ModuleMarker>,
}

impl Deployer {
    pub fn new(
        multiprogress: Arc<MultiProgress>,
        queue: Arc<ArrayQueue<usize>>,
        deployment_plan: Option<Arc<ModuleDeploymentPlan>>,
    ) -> Self {
        Self {
            multiprogress,
            queue,
            deployment_plan,
        }
    }

    pub fn do_work(
        &self,
        modules: &[ModuleToDeploy],
        cfg: &ClientConfig,
        deploy_opts: &DeployOptions,
    ) -> Result<()> {
        // Consume modules from the shared queue and deploy them.
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
            InnerDefinition::Task(ref task) => {
                self.deploy_task(task, deploy_opts, cfg)
            }
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
        let spin_opt = SpinnerOptions::new(message);

        let pb = self.multiprogress.add(ProgressBar::new(std::u64::MAX));
        let wu = WaitUntil::new_multi(&spin_opt, pb);
        let deploy_result = wu.spin_until_status(|| {
            let result =
                request::deploy_module(module, deploy_opts, &cfg.daemon_url)?;

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
        let spin_opt = SpinnerOptions::new(message);
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
        deploy_opts: &DeployOptions,
        cfg: &ClientConfig,
    ) -> Result<()> {
        let message = format!("Running task {}", cbold!(&module.name));
        let spin_opt = SpinnerOptions::new(message);

        let pb = self.multiprogress.add(ProgressBar::new(std::u64::MAX));
        let wu = WaitUntil::new_multi(&spin_opt, pb);
        let force = deploy_opts.force_deploy;

        // If none of this tasks services will be deployed then skip deploying
        // this task also.
        let skipped_by_plan = !self.should_deploy(module.name.as_str(), force);
        wu.spin_until_status(|| {
            if skipped_by_plan {
                return Ok(WaitResult::from(
                    false,
                    cdim!("(Skipping)").to_string(),
                ));
            }
            let result =
                request::deploy_task(module, deploy_opts, &cfg.daemon_url)?;
            let status = csuccess!("(Done)").to_string();
            Ok(WaitResult::from(result.success, status))
        })?;

        Ok(())
    }

    fn deploy_group(&self, module: &GroupDefinition) {
        let message = format!("Group {}", cbold!(&module.name));
        let spin_opt = SpinnerOptions::new(message);

        let pb = self.multiprogress.add(ProgressBar::new(std::u64::MAX));
        let mut ws = WaitSpin::from(&spin_opt, pb);
        ws.stop_with_status(csuccess!("(Done)").to_string());
    }

    fn should_deploy(&self, module_name: &str, force: bool) -> bool {
        if force {
            true
        } else if let Some(deployment_plan) = &self.deployment_plan {
            *deployment_plan
                .should_deploy
                .get(module_name)
                .unwrap_or(&true)
        } else {
            true
        }
    }

    pub fn perform_check(check_def: &CheckDefinition) -> Result<()> {
        let message =
            format!("Check {} ({})", cbold!(&check_def.about), check_def.name);
        let spin_opt = SpinnerOptions::new(message);
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
            if let Some(suggested_fix) = &check_def.suggested_fix {
                Self::ask_to_apply_suggested_fix(check_def, suggested_fix);
            } else {
                bail!(
                    "The {} check has failed\n\
                    {}: {}",
                    cbold!(&check_def.about),
                    cbold!("Message"),
                    check_def.help
                )
            }
        }
        Ok(())
    }

    fn ask_to_apply_suggested_fix(
        check_def: &CheckDefinition,
        suggested_fix: &SuggestedFixDefinition,
    ) {
        tprint!(
            "{} The {} check has failed\n {}: {}\n",
            cfail!("Error:"),
            cbold!(&check_def.about),
            cbold!("Message"),
            check_def.help
        );

        tprint!(
            "{} There is a suggested fix available. \n {} {}\n\n {} (y/n)",
            YELLOW_NOTEBOOK,
            cbold!("Fix details:"),
            suggested_fix.message,
            cbold!("Would you like to apply it?"),
        );
        loop {
            let line: String = read!("{}\n");
            if line.to_lowercase() == "y" {
                tprint!("{} {}", HOUR_GLASS, cbold!("Applying..."));
                if apply_suggested_fix(suggested_fix).is_ok() {
                    tprint!(
                        "{} {}",
                        SUCCESS,
                        format!(
                            "{} {}",
                            csuccess!("Fix applied for:"),
                            cbold!(&check_def.about)
                        )
                    );
                } else {
                    texit!("Suggested fix failed to apply");
                }
                break;
            } else if line.to_lowercase() == "n" {
                texit!("Resolve the check manually and try again.");
            }
        }
    }

    pub fn run_checks<T: AsRef<ModuleDefinition>>(
        checks_map: HashMap<String, CheckDefinition>,
        modules: &[T],
    ) -> Result<()> {
        let mut already_performed = HashSet::new();
        for m in modules {
            let checks = match &m.as_ref().inner {
                InnerDefinition::Group(grp) => grp.checks.as_slice(),
                InnerDefinition::Service(srvc) => srvc.checks.as_slice(),
                InnerDefinition::Task(tsk) => tsk.checks.as_slice(),
                _ => &[],
            };

            for check in checks {
                let check = checks_map
                    .get(check)
                    .ok_or_else(|| anyhow!("Check '{}' not defined", check))?;

                if !already_performed.contains(&check.name) {
                    Self::perform_check(check)?;
                    already_performed.insert(check.name.clone());
                }
            }
        }
        Ok(())
    }

    fn is_going_to_deploy(
        plan_response: &ApiGetPlanResponse,
        module_name: &str,
    ) -> bool {
        match plan_response.plan.get(module_name) {
            Some(action) => match action {
                ApiPlannedAction::WillDeploy => true,
                ApiPlannedAction::WillRedeploy => true,
                ApiPlannedAction::AlreadyDeployed => false,
            },
            None => true,
        }
    }

    pub fn obtain_plan(
        modules: &[&DependencyNode<&ModuleDefinition, ModuleMarker>],
        cfg: &ClientConfig,
        deploy_opts: &DeployOptions,
    ) -> Result<ModuleDeploymentPlan> {
        let module_defs: Vec<_> = modules.iter().map(|m| m.value).collect();
        let plan = get_plan(&module_defs, deploy_opts, &cfg.daemon_url)?;

        let should_deploy = modules
            .iter()
            .filter_map(|module| match &module.value.inner {
                InnerDefinition::Service(svc) => {
                    // This is currently unused (the client will always attempt
                    // to deploy a service but the daemon will skip if it's
                    // already deployed). Maybe considering skipping anyway to
                    // avoid the net request.
                    let should_deploy =
                        Self::is_going_to_deploy(&plan, &svc.name);
                    Some((svc.name.clone(), should_deploy))
                }
                InnerDefinition::Task(tsk) => {
                    // A task should deploy if any of its originating services
                    // will deploy.
                    let any_origin_deploys =
                        module.origin_nodes.iter().any(|origin_node| {
                            Self::is_going_to_deploy(&plan, origin_node)
                        });
                    Some((tsk.name.clone(), any_origin_deploys))
                }
                _ => None,
            })
            .collect();

        Ok(ModuleDeploymentPlan { should_deploy })
    }
}
