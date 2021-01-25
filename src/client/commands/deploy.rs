use crate::client::cli::ClientConfig;
use crate::client::config::read_module_definitions;
use crate::client::emoji::{LINK, LOOKING_GLASS, SUCCESS, TEXTBOOK, VAN};
use crate::client::module::{module_names_set, remove_checks};
use crate::client::module::{
    CheckDefinition, GroupDefinition, InnerDefinition, ModuleDefinition,
    ModuleMarker, ServiceOrTaskDefinition,
};
use crate::client::process::run_check;
use crate::client::progress::{SpinnerOptions, WaitResult, WaitUntil};
use crate::client::request;
use crate::client::validation::validate_modules_selected;
use crate::daemon::api::ApiHealthStatus;
use crate::dependency::{DependencyGraph, DependencyNode};
use anyhow::{anyhow, bail, Result};
use clap::ArgMatches;
use std::collections::{HashMap, HashSet};
use std::thread;
use std::time::Duration;

pub struct DeployOptions {
    force_deploy: bool,
    skip_checks: bool,
    only_selected: bool,
    skip_healthchecks: bool,
}

impl DeployOptions {
    pub fn from(opts: &ArgMatches) -> DeployOptions {
        let force_deploy = opts.is_present("force");
        let skip_healthchecks = opts.is_present("skip_healthchecks");
        let skip_checks = opts.is_present("skip_checks");

        let only_selected = opts.is_present("only_selected");
        Self {
            force_deploy,
            skip_healthchecks,
            skip_checks,
            only_selected,
        }
    }
}

pub fn deploy_cmd(
    modules_to_deploy: Vec<&str>,
    cfg: &ClientConfig,
    deploy_opts: &DeployOptions,
) -> Result<()> {
    tprintstep!("Looking for module definitions...", 1, 5, LOOKING_GLASS);
    let mut module_defs = read_module_definitions(&cfg)?;
    let checks_map = remove_checks(&mut module_defs);
    let module_names = module_names_set(&module_defs);

    validate_modules_selected(&module_names, &modules_to_deploy)?;

    let dep_graph: DependencyGraph<_, _>;

    let deployed: Vec<_> = if !deploy_opts.only_selected {
        tprintstep!("Resolving dependencies...", 2, 5, LINK);
        dep_graph = DependencyGraph::from(&module_defs, &modules_to_deploy);
        let sorted = dep_graph.dependency_sort()?;

        run_checks(checks_map, &sorted, deploy_opts)?;

        tprintstep!("Deploying...", 4, 5, VAN);
        deploy_with_dependencies(&sorted, cfg, deploy_opts)?;
        sorted.iter().map(|d| &d.key).collect()
    } else {
        let msg = format!("Resolving dependencies... {}", cdim!("(Skip)"));
        tprintstep!(msg, 2, 5, LINK);
        let modules_to_deploy_set: HashSet<_> =
            modules_to_deploy.iter().copied().collect();

        let selected: Vec<_> = module_defs
            .iter()
            .filter(|m| modules_to_deploy_set.contains(m.name.as_str()))
            .collect();

        run_checks(checks_map, &selected, deploy_opts)?;
        tprintstep!("Deploying...", 4, 5, VAN);
        deploy_without_dependencies(&selected, cfg, deploy_opts)?;
        selected.iter().map(|m| &m.name).collect()
    };

    let deploy_txt =
        format!("{}: {:?}", csuccess!("Deployed modules"), deployed);
    tprintstep!(deploy_txt, 5, 5, SUCCESS);
    Ok(())
}

fn run_checks<T: AsRef<ModuleDefinition>>(
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
                let check = checks_map
                    .get(check)
                    .ok_or_else(|| anyhow!("Check '{}' not defined", check))?;

                perform_check(check)?;
            }
        }
    }
    Ok(())
}

fn perform_check(check_def: &CheckDefinition) -> Result<()> {
    let message =
        format!("Check {} ({})", cbold!(&check_def.about), check_def.name);
    let spin_opt = SpinnerOptions::new(message).clear_on_finish(false);
    let mut wu = WaitUntil::new(&spin_opt);

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

fn deploy_with_dependencies(
    sorted: &[&DependencyNode<&ModuleDefinition, ModuleMarker>],
    cfg: &ClientConfig,
    deploy_opts: &DeployOptions,
) -> Result<()> {
    for m in sorted {
        match m.value.inner {
            InnerDefinition::Task(ref task) => deploy_task(task, cfg),
            InnerDefinition::Service(ref service) => {
                deploy_and_maybe_wait_service(
                    service,
                    m.marker,
                    cfg,
                    deploy_opts,
                )
            }
            InnerDefinition::Group(ref group) => {
                deploy_group(group);
                Ok(())
            }
            InnerDefinition::Check(_) => Ok(()),
        }?;
    }
    Ok(())
}

fn deploy_without_dependencies(
    sorted: &[&ModuleDefinition],
    cfg: &ClientConfig,
    deploy_opts: &DeployOptions,
) -> Result<()> {
    for m in sorted {
        match m.inner {
            InnerDefinition::Task(ref task) => deploy_task(task, cfg),
            InnerDefinition::Service(ref service) => {
                deploy_and_maybe_wait_service(
                    service,
                    Some(ModuleMarker::Instant),
                    cfg,
                    deploy_opts,
                )
            }
            InnerDefinition::Group(ref group) => {
                deploy_group(group);
                Ok(())
            }
            InnerDefinition::Check(_) => Ok(()),
        }?;
    }
    Ok(())
}

fn deploy_and_maybe_wait_service(
    service: &ServiceOrTaskDefinition,
    marker: Option<ModuleMarker>,
    cfg: &ClientConfig,
    deploy_opts: &DeployOptions,
) -> Result<()> {
    let monitor_handle = deploy_service(service, cfg, deploy_opts)?;
    if let Some(handle) = monitor_handle {
        if (marker == Some(ModuleMarker::WaitHealthcheck)
            || service.always_wait_healthcheck)
            && !deploy_opts.skip_healthchecks
        {
            wait_until_healthy(service.name.as_str(), handle.as_str(), cfg)?;
        }
    }
    Ok(())
}

fn deploy_service(
    module: &ServiceOrTaskDefinition,
    cfg: &ClientConfig,
    deploy_opts: &DeployOptions,
) -> Result<Option<String>> {
    let message = format!("Deploying {}", cbold!(&module.name));
    let spin_opt = SpinnerOptions::new(message).clear_on_finish(false);

    let mut wu = WaitUntil::new(&spin_opt);
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
    module_name: &str,
    monitor_handle: &str,
    cfg: &ClientConfig,
) -> Result<()> {
    let message = format!("Waiting {} to be healthy", cbold!(module_name));
    let spin_opt = SpinnerOptions::new(message).clear_on_finish(false);
    let mut wu = WaitUntil::new(&spin_opt);

    wu.spin_until_status(|| loop {
        let status = csuccess!("(Done)").to_string();
        match request::poll_health(monitor_handle, &cfg.daemon_url)?
            .healthcheck_status
        {
            Some(ApiHealthStatus::Successful) => {
                break Ok(WaitResult::from((), status))
            }
            Some(ApiHealthStatus::RetriesExceeded) => {
                bail!(
                    "The service did not complete its healthcheck in time.\n\
                       Check the logs for more details."
                )
            }
            Some(ApiHealthStatus::Error) => {
                bail!(
                    "An error occured while waiting for the service \
                    healthcheck to complete.\nThis is usually a mistake in \
                    the healthcheck configuration, ensure the command or \
                    condition is correct."
                )
            }
            Some(ApiHealthStatus::Pending) | None => {
                thread::sleep(Duration::from_secs(2));
            }
        }
    })?;

    Ok(())
}

fn deploy_task(
    module: &ServiceOrTaskDefinition,
    cfg: &ClientConfig,
) -> Result<()> {
    let message = format!("Running task {}", cbold!(&module.name));
    let spin_opt = SpinnerOptions::new(message).clear_on_finish(false);

    let mut wu = WaitUntil::new(&spin_opt);
    wu.spin_until_status(|| {
        let result = request::deploy_task(module, &cfg.daemon_url)?;
        let status = csuccess!("(Done)").to_string();
        Ok(WaitResult::from(result, status))
    })?;

    Ok(())
}

fn deploy_group(module: &GroupDefinition) {
    let message = format!("Group {}", cbold!(&module.name));
    tiprint!(
        10, // indent level
        "{} {}",
        message,
        csuccess!("(Done)")
    );
}
