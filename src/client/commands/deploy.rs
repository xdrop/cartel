use crate::client::cli::CliOptions;
use crate::client::config::read_module_definitions;
use crate::client::emoji::{
    HOUR_GLASS, LINK, LOOKING_GLASS, SUCCESS, TEXTBOOK, VAN,
};
use crate::client::module::{checks_index, module_names_set};
use crate::client::module::{
    CheckDefinitionV1, ModuleKindV1, ServiceOrTaskDefinitionV1,
};
use crate::client::process::run_check;
use crate::client::progress::{SpinnerOptions, WaitUntil};
use crate::client::request;
use crate::client::validation::validate_modules_selected;
use crate::daemon::api::ApiDeploymentResponse;
use crate::dependency::DependencyGraph;
use anyhow::{bail, Result};
use console::style;

pub fn deploy_cmd(
    modules_to_deploy: Vec<&str>,
    cli_config: &CliOptions,
) -> Result<()> {
    tprintstep!("Looking for module definitions...", 1, 5, LOOKING_GLASS);
    let (module_defs, check_defs) = read_module_definitions()?;
    let module_names = module_names_set(&module_defs);
    let checks_index = checks_index(&check_defs);

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
            for check in &m.checks {
                // TODO: Handle error
                do_check(checks_index.get(check.as_str()).unwrap())?;
            }
        }
    }

    tprintstep!("Deploying...", 4, 5, VAN);

    for m in &ordered {
        match m.kind {
            ModuleKindV1::Task => deploy_task(m, cli_config),
            ModuleKindV1::Service => {
                deploy_service(m, &module_defs, cli_config)
            }
            ModuleKindV1::Check => Ok(()),
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

fn do_check(check_def: &CheckDefinitionV1) -> Result<()> {
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
    module_defs: &Vec<ServiceOrTaskDefinitionV1>,
    cli_config: &CliOptions,
) -> Result<()> {
    let message = format!("Deploying {}", style(&module.name).white().bold());
    let spin_opt = SpinnerOptions::new(message.clone()).clear_on_finish(false);

    let mut wu = WaitUntil::new(&spin_opt);
    let deploy_result = wu.spin_until(|| {
        request::deploy_modules(
            &vec![&module.name],
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
    let deploy_result =
        wu.spin_until(|| request::deploy_task(module, &cli_config.daemon_url))?;

    tiprint!(
        10, // indent level
        "{} {}",
        message,
        style("(Done)").green().bold()
    );
    Ok(())
}
