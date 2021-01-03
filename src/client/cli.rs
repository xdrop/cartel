use super::commands::*;
use anyhow::{anyhow, bail, Result};
use clap::{App, Arg, ArgMatches, SubCommand};
use std::env;

pub struct CliOptions {
    pub verbose: u64,
    pub pager_cmd: Vec<String>,
    pub daemon_url: String,
    pub skip_checks: bool,
}

pub fn cli_app() -> Result<()> {
    let matches = App::new("cartel")
        .version("0.1.1-alpha")
        .about("Panayiotis P. <xdrop.me@gmail.com>")
        .about("Service orchestration made easy")
        .arg(
            Arg::with_name("config")
                .short("c")
                .long("config")
                .value_name("FILE")
                .help("Sets a custom config file")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("verbose")
                .short("v")
                .multiple(true)
                .help("Sets the level of verbosity"),
        )
        .arg(
            Arg::with_name("skip_checks")
                .short("nc")
                .long("skip checks")
                .help("Disables running checks"),
        )
        .subcommand(
            SubCommand::with_name("deploy")
                .about("Deploys a module (and it's dependencies)")
                .arg(
                    Arg::with_name("modules")
                        .help("Modules to deploy")
                        .multiple(true)
                        .takes_value(true),
                ),
        )
        .subcommand(
            SubCommand::with_name("run")
                .about("Runs a task (but NOT it's dependencies)")
                .arg(
                    Arg::with_name("task")
                        .help("The task ro run")
                        .multiple(false)
                        .takes_value(true),
                ),
        )
        .subcommand(
            SubCommand::with_name("ps")
                .about("Print currently running services"),
        )
        .subcommand(
            SubCommand::with_name("logs")
                .about("Print logs of a service")
                .after_help(
                    "By default the 'less' pager is used. To change this, \
                    set the CARTEL_PAGER environment variable.",
                )
                .arg(
                    Arg::with_name("service")
                        .help("The service to print the logs of")
                        .takes_value(true),
                ),
        )
        .subcommand(
            SubCommand::with_name("stop")
                .about("Stop a running service")
                .arg(
                    Arg::with_name("service")
                        .help("Service to stop")
                        .takes_value(true),
                ),
        )
        .subcommand(
            SubCommand::with_name("restart")
                .about("Restart a service")
                .arg(
                    Arg::with_name("service")
                        .help("Service to restart")
                        .takes_value(true),
                ),
        )
        .get_matches();

    let cli_config = cli_config(&matches)?;
    invoke_subcommand(&matches, &cli_config)?;
    Ok(())
}

fn cli_config(matches: &ArgMatches) -> Result<CliOptions> {
    let pager_cmd_str =
        env::var("CARTEL_PAGER").unwrap_or("less +F -XRS -~".to_string());
    let pager_cmd: Vec<String> =
        pager_cmd_str.split(' ').map(|s| s.to_string()).collect();

    if pager_cmd.is_empty() || pager_cmd_str.is_empty() {
        bail!(
            "Invalid log pager specified. \
            Are you overriding CARTEL_PAGER?"
        );
    }

    Ok(CliOptions {
        verbose: matches.occurrences_of("v"),
        skip_checks: matches.is_present("skip_checks"),
        pager_cmd,
        // TODO: Make config
        daemon_url: "http://localhost:8000/api/v1".to_string(),
    })
}

fn invoke_subcommand(
    matches: &ArgMatches,
    cli_config: &CliOptions,
) -> Result<()> {
    match matches.subcommand() {
        ("deploy", Some(deploy_cli_opts)) => {
            let modules_to_deploy = deploy_cli_opts
                .values_of("modules")
                .ok_or_else(|| anyhow!("Expected at least one module"))?
                .collect();
            deploy_cmd(modules_to_deploy, cli_config)?;
        }
        ("run", Some(run_cli_opts)) => {
            let task_name = run_cli_opts
                .value_of("task")
                .ok_or_else(|| anyhow!("Expected task name"))?;
            run_task_cmd(task_name, cli_config)?;
        }
        ("ps", Some(_)) => {
            list_modules_cmd(cli_config)?;
        }
        ("stop", Some(stop_cli_opts)) => {
            let module_to_stop = stop_cli_opts
                .value_of("service")
                .ok_or_else(|| anyhow!("Expected service name"))?;
            stop_module_cmd(module_to_stop, cli_config)?;
        }
        ("restart", Some(restart_cli_opts)) => {
            let module_to_restart = restart_cli_opts
                .value_of("service")
                .ok_or_else(|| anyhow!("Expected service name"))?;
            restart_module_cmd(module_to_restart, cli_config)?;
        }
        ("logs", Some(logs_cli_opts)) => {
            let module_name = logs_cli_opts
                .value_of("service")
                .ok_or_else(|| anyhow!("Expected service name"))?;
            print_logs(module_name, cli_config)?;
        }
        _ => {}
    }
    Ok(())
}
