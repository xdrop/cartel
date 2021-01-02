use crate::client::cli::CliOptions;
use crate::client::config::read_module_definitions;
use crate::client::emoji::{HOUR_GLASS, LOOKING_GLASS, SUCCESS, UP_ARROW};
use crate::client::module::module_names_set;
use crate::client::module::{ModuleDefinitionV1, ModuleKindV1};
use crate::client::progress::{SpinnerOptions, WaitUntil};
use crate::client::request;
use crate::client::validation::validate_modules_selected;
use crate::daemon::api::ApiDeploymentResponse;
use crate::dependency::DependencyGraph;
use anyhow::Result;
use console::style;

pub fn deploy_cmd(
    modules_to_deploy: Vec<&str>,
    cli_config: &CliOptions,
) -> Result<()> {
    tprintstep!("Looking for module definitions...", 1, 4, LOOKING_GLASS);
    let module_defs = read_module_definitions()?;
    let module_names = module_names_set(&module_defs);

    tprintstep!("Resolving dependencies...", 2, 4, UP_ARROW);

    validate_modules_selected(&module_names, &modules_to_deploy)?;

    let dependency_graph =
        DependencyGraph::from(&module_defs, &modules_to_deploy);
    let ordered = dependency_graph.dependency_sort()?;
    tprintstep!("Deploying...", 3, 4, HOUR_GLASS);

    for m in &ordered {
        match m.kind {
            ModuleKindV1::Task => deploy_task(m, cli_config),
            ModuleKindV1::Service => {
                deploy_service(m, &module_defs, cli_config)
            }
        }?;
    }

    let deploy_txt = format!(
        "{}: {:?}",
        style("Deployed modules").bold().green(),
        &ordered.iter().map(|m| &m.name).collect::<Vec<_>>()
    );
    tprintstep!(deploy_txt, 4, 4, SUCCESS);
    Ok(())
}

fn deploy_service(
    module: &ModuleDefinitionV1,
    module_defs: &Vec<ModuleDefinitionV1>,
    cli_config: &CliOptions,
) -> Result<()> {
    let spin_opt = SpinnerOptions::new(format!("Deploying {}", module.name))
        .clear_on_finish(true);

    let mut wu = WaitUntil::new(&spin_opt);
    let deploy_result = wu.spin_until(|| {
        request::deploy_modules(
            &vec![&module.name],
            module_defs,
            &cli_config.daemon_url,
        )
    })?;

    let deploy_status = if deploy_result.deployed[&module.name] {
        style("(Deployed)").white().dim().bold()
    } else {
        style("(Already deployed)").white().dim().bold()
    };

    tiprint!(
        10, // indent level
        "Deploying {} {}",
        module.name,
        deploy_status
    );
    Ok(())
}

fn deploy_task(
    module: &ModuleDefinitionV1,
    cli_config: &CliOptions,
) -> Result<()> {
    let spin_opt = SpinnerOptions::new(format!("Running task {}", module.name))
        .clear_on_finish(true);

    let mut wu = WaitUntil::new(&spin_opt);
    let deploy_result =
        wu.spin_until(|| request::deploy_task(module, &cli_config.daemon_url))?;

    tiprint!(
        10, // indent level
        "Running task {} {}",
        module.name,
        style("(Done)").white().dim().bold()
    );
    Ok(())
}
