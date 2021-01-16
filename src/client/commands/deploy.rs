use crate::client::cli::CliOptions;
use crate::client::config::read_module_definitions;
use crate::client::emoji::{LINK, LOOKING_GLASS, SUCCESS, TEXTBOOK, VAN};
use crate::client::module::{module_names_set, remove_checks};
use crate::client::module::{
    CheckDefinition, GroupDefinition, InnerDefinition, ModuleDefinition,
    ModuleMarker, ServiceOrTaskDefinition,
};
use crate::client::process::run_check;
use crate::client::progress::{SpinnerOptions, WaitUntil};
use crate::client::request;
use crate::client::validation::validate_modules_selected;
use crate::daemon::api::ApiHealthStatus;
use crate::dependency::{DependencyGraph, DependencyNode};
use anyhow::{anyhow, bail, Result};
use console::style;
use std::collections::HashMap;
use std::thread;
use std::time::Duration;

pub fn deploy_cmd(
    modules_to_deploy: Vec<&str>,
    cli_config: &CliOptions,
) -> Result<()> {
    tprintstep!("Looking for module definitions...", 1, 5, LOOKING_GLASS);
    let mut module_defs = read_module_definitions(&cli_config)?;
    let checks_map = remove_checks(&mut module_defs);
    let module_names = module_names_set(&module_defs);

    tprintstep!("Resolving dependencies...", 2, 5, LINK);

    validate_modules_selected(&module_names, &modules_to_deploy)?;

    let dependency_graph =
        DependencyGraph::from(&module_defs, &modules_to_deploy);
    let ordered = dependency_graph.dependency_sort()?;

    run_checks(checks_map, &ordered, cli_config)?;

    tprintstep!("Deploying...", 4, 5, VAN);

    for m in &ordered {
        match m.value.inner {
            InnerDefinition::Task(ref task) => deploy_task(task, cli_config),
            InnerDefinition::Service(ref service) => {
                let monitor_handle = deploy_service(service, cli_config)?;
                if let Some(handle) = monitor_handle {
                    if m.marker != Some(ModuleMarker::Instant)
                        || service.always_wait_healthcheck
                    {
                        wait_until_healthy(
                            service.name.as_str(),
                            handle.as_str(),
                            cli_config,
                        )?;
                    }
                }
                Ok(())
            }
            InnerDefinition::Group(ref group) => {
                deploy_group(group);
                Ok(())
            }
            InnerDefinition::Check(_) => Ok(()),
        }?;
    }

    let deploy_txt = format!(
        "{}: {:?}",
        style("Deployed modules").bold().green(),
        &ordered.iter().map(|m| &m.value.name).collect::<Vec<_>>()
    );
    tprintstep!(deploy_txt, 5, 5, SUCCESS);
    Ok(())
}

fn run_checks(
    checks_map: HashMap<String, CheckDefinition>,
    modules: &Vec<&DependencyNode<&ModuleDefinition, ModuleMarker>>,
    cli_config: &CliOptions,
) -> Result<()> {
    if cli_config.skip_checks {
        let msg = format!(
            "Running checks... {}",
            style("(Skip)").bold().white().dim()
        );
        tprintstep!(msg, 3, 5, TEXTBOOK);
    } else {
        tprintstep!("Running checks...", 3, 5, TEXTBOOK);
        for m in modules {
            let checks = match &m.value.inner {
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
    let message = format!(
        "Check {} ({})",
        style(&check_def.about).white().bold(),
        check_def.name
    );
    let spin_opt = SpinnerOptions::new(message.clone()).clear_on_finish(false);
    let mut wu = WaitUntil::new(&spin_opt);
    let check_result = wu.spin_until(|| run_check(check_def))?;
    if check_result.success() {
        tiprint!(10, "{} {}", message, style("(OK)").green().bold());
    } else {
        tiprint!(10, "{} {}", message, style("(FAIL)").red().bold());
        bail!(
            "The {} check has failed\n\
            {}: {}",
            style(&check_def.about).white().bold(),
            style("Message").white().bold(),
            check_def.help
        )
    }
    Ok(())
}

fn deploy_service(
    module: &ServiceOrTaskDefinition,
    cli_config: &CliOptions,
) -> Result<Option<String>> {
    let message = format!("Deploying {}", style(&module.name).white().bold());
    let spin_opt = SpinnerOptions::new(message.clone()).clear_on_finish(false);

    let mut wu = WaitUntil::new(&spin_opt);
    let deploy_result = wu.spin_until(|| {
        request::deploy_modules(module, &cli_config.daemon_url)
    })?;

    let deploy_status = if deploy_result.deployed {
        style("(Deployed)").green().bold()
    } else {
        style("(Already deployed)").white().dim().bold()
    };

    let monitor_handle = deploy_result.monitor;

    tiprint!(
        10, // indent level
        "{} {}",
        message,
        deploy_status,
    );

    Ok(monitor_handle)
}

fn wait_until_healthy(
    module_name: &str,
    monitor_handle: &str,
    cli_config: &CliOptions,
) -> Result<()> {
    let message = format!(
        "Waiting {} to be healthy",
        style(module_name).white().bold()
    );
    let spin_opt = SpinnerOptions::new(message.clone()).clear_on_finish(false);
    let mut wu = WaitUntil::new(&spin_opt);

    wu.spin_until(|| loop {
        match request::poll_health(monitor_handle, &cli_config.daemon_url)?
            .healthcheck_status
        {
            Some(ApiHealthStatus::Successful) => break Ok(()),
            Some(ApiHealthStatus::RetriesExceeded) => {
                bail!(
                    "The service did not complete its healthcheck in time.\n\
                       Check the logs for more details."
                )
            }
            Some(ApiHealthStatus::Pending) | None => {
                thread::sleep(Duration::from_secs(2));
            }
        }
    })?;

    tiprint!(
        10, // indent level
        "{} {}",
        message,
        style("(Done)").green().bold()
    );
    Ok(())
}

fn deploy_task(
    module: &ServiceOrTaskDefinition,
    cli_config: &CliOptions,
) -> Result<()> {
    let message =
        format!("Running task {}", style(&module.name).white().bold());
    let spin_opt = SpinnerOptions::new(message.clone()).clear_on_finish(false);

    let mut wu = WaitUntil::new(&spin_opt);
    wu.spin_until(|| request::deploy_task(module, &cli_config.daemon_url))?;

    tiprint!(
        10, // indent level
        "{} {}",
        message,
        style("(Done)").green().bold()
    );
    Ok(())
}

fn deploy_group(module: &GroupDefinition) {
    let message = format!("Group {}", style(&module.name).white().bold());
    tiprint!(
        10, // indent level
        "{} {}",
        message,
        style("(Done)").green().bold()
    );
}
