use crate::client::cli::CliOptions;
use crate::client::config::read_module_definitions;
use crate::client::emoji::{LINK, LOOKING_GLASS, SUCCESS, TEXTBOOK, VAN};
use crate::client::module::{filter_services, module_names_set, remove_checks};
use crate::client::module::{
    CheckDefinitionV1, InnerDefinitionV1, ServiceOrTaskDefinitionV1,
};
use crate::client::process::run_check;
use crate::client::progress::{SpinnerOptions, WaitUntil};
use crate::client::request;
use crate::client::validation::validate_modules_selected;
use crate::dependency::DependencyGraph;
use anyhow::{anyhow, bail, Result};
use console::style;

pub fn deploy_cmd(
    modules_to_deploy: Vec<&str>,
    cli_config: &CliOptions,
) -> Result<()> {
    tprintstep!("Looking for module definitions...", 1, 5, LOOKING_GLASS);
    let mut module_defs = read_module_definitions()?;
    let checks_map = remove_checks(&mut module_defs);
    let module_names = module_names_set(&module_defs);
    let services = filter_services(&module_defs);

    tprintstep!("Resolving dependencies...", 2, 5, LINK);

    validate_modules_selected(&module_names, &modules_to_deploy)?;

    let dependency_graph =
        DependencyGraph::from(&module_defs, &modules_to_deploy);
    let ordered = dependency_graph.dependency_sort()?;

    if cli_config.skip_checks {
        let msg = format!(
            "Running checks... {}",
            style("(Skip)").bold().white().dim()
        );
        tprintstep!(msg, 3, 5, TEXTBOOK);
    } else {
        tprintstep!("Running checks...", 3, 5, TEXTBOOK);
        for m in &ordered {
            let checks = match &m.inner {
                InnerDefinitionV1::Group(grp) => grp.checks.as_slice(),
                InnerDefinitionV1::Service(srvc) => srvc.checks.as_slice(),
                InnerDefinitionV1::Task(tsk) => tsk.checks.as_slice(),
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

    tprintstep!("Deploying...", 4, 5, VAN);

    for m in &ordered {
        match m.inner {
            InnerDefinitionV1::Task(ref task) => deploy_task(task, cli_config),
            InnerDefinitionV1::Service(ref service) => {
                deploy_service(service, services.as_slice(), cli_config)
            }
            InnerDefinitionV1::Check(_) => Ok(()),
            InnerDefinitionV1::Group(_) => Ok(()),
        }?;
    }

    let deploy_txt = format!(
        "{}: {:?}",
        style("Deployed modules").bold().green(),
        &ordered.iter().map(|m| &m.name).collect::<Vec<_>>()
    );
    tprintstep!(deploy_txt, 5, 5, SUCCESS);
    Ok(())
}

fn perform_check(check_def: &CheckDefinitionV1) -> Result<()> {
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
    module: &ServiceOrTaskDefinitionV1,
    module_defs: &[&ServiceOrTaskDefinitionV1],
    cli_config: &CliOptions,
) -> Result<()> {
    let message = format!("Deploying {}", style(&module.name).white().bold());
    let spin_opt = SpinnerOptions::new(message.clone()).clear_on_finish(false);

    let mut wu = WaitUntil::new(&spin_opt);
    let deploy_result = wu.spin_until(|| {
        request::deploy_modules(
            &[&module.name],
            module_defs,
            &cli_config.daemon_url,
        )
    })?;

    let deploy_status = if deploy_result.deployed[&module.name] {
        style("(Deployed)").green().bold()
    } else {
        style("(Already deployed)").white().dim().bold()
    };

    tiprint!(
        10, // indent level
        "{} {}",
        message,
        deploy_status
    );
    Ok(())
}

fn deploy_task(
    module: &ServiceOrTaskDefinitionV1,
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
