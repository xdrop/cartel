use super::commands::*;
use anyhow::{anyhow, bail, Result};
use clap::{crate_version, App, AppSettings, Arg, ArgMatches, SubCommand};
use std::env;

pub struct CliOptions {
    pub verbose: u64,
    pub module_file: Option<String>,
    pub default_pager_cmd: Vec<String>,
    pub full_pager_cmd: Vec<String>,
    pub follow_pager_cmd: Vec<String>,
    pub daemon_url: String,
    pub skip_checks: bool,
}

pub fn cli_app() -> Result<()> {
    let matches = App::new("cartel")
        .version(&crate_version!()[..])
        .about("Panayiotis P. <xdrop.me@gmail.com>")
        .about("Service orchestration made easy")
        .setting(AppSettings::SubcommandRequiredElseHelp)
        .arg(
            Arg::with_name("config")
                .short("c")
                .long("config")
                .value_name("FILE")
                .help("Sets a custom config file")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("file")
                .short("f")
                .long("file")
                .value_name("FILE")
                .help("Specify a module definitions file to read")
                .takes_value(true)
                .multiple(false),
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
                .visible_alias("d")
                .arg(
                    Arg::with_name("modules")
                        .help("Modules to deploy")
                        .multiple(true)
                        .takes_value(true),
                ),
        )
        .subcommand(
            SubCommand::with_name("run")
                .visible_alias("r")
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
                .visible_alias("l")
                .about("Print logs of a service")
                .after_help(
                    "By default `tail` and `less` are used to page logs \
                    (depending on the option). Each can be controlled from \
                    an environment variable. \n\n\
                    You can do this by overriding \
                    CARTEL_DEFAULT_LOG_PAGER, \
                    CARTEL_FULL_LOG_PAGER or CARTEL_FOLLOW_LOG_PAGER.",
                )
                .arg(
                    Arg::with_name("follow")
                        .long("follow")
                        .short("f")
                        .help("Print the full logs in follow mode")
                        .takes_value(false),
                )
                .arg(
                    Arg::with_name("all")
                        .long("all")
                        .short("a")
                        .conflicts_with("follow")
                        .help("Print the full logs")
                        .takes_value(false),
                )
                .arg(
                    Arg::with_name("service")
                        .help("The service to print the logs of")
                        .takes_value(true),
                ),
        )
        .subcommand(
            SubCommand::with_name("stop")
                .visible_alias("s")
                .about("Stop a running service")
                .arg(
                    Arg::with_name("services")
                        .help("Services to stop")
                        .multiple(true)
                        .takes_value(true),
                ),
        )
        .subcommand(
            SubCommand::with_name("down").about("Stop all running services"),
        )
        .subcommand(
            SubCommand::with_name("restart")
                .visible_alias("rr")
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
    let full_pager_cmd = parse_cmd_from_env("CARTEL_FULL_LOG_PAGER", "less")?;
    let default_pager_cmd =
        parse_cmd_from_env("CARTEL_DEFAULT_LOG_PAGER", "tail -f -n 30")?;
    let follow_pager_cmd =
        parse_cmd_from_env("CARTEL_FOLLOW_LOG_PAGER", "less +F")?;

    Ok(CliOptions {
        verbose: matches.occurrences_of("v"),
        skip_checks: matches.is_present("skip_checks"),
        module_file: matches.value_of("file").map(String::from),
        default_pager_cmd,
        full_pager_cmd,
        follow_pager_cmd,
        // TODO: Make config
        daemon_url: "http://localhost:8000/api/v1".to_string(),
    })
}

fn parse_cmd_from_env(env: &str, default: &str) -> Result<Vec<String>> {
    #[allow(clippy::or_fun_call)]
    let cmd_str = env::var(env).unwrap_or(default.to_string());
    let cmd: Vec<String> = cmd_str.split(' ').map(|s| s.to_string()).collect();

    if cmd.is_empty() || cmd_str.is_empty() {
        bail!(
            "Invalid command specified \
            Are you overriding {}?",
            env
        );
    }

    Ok(cmd)
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
            let modules_to_stop = stop_cli_opts
                .values_of("services")
                .ok_or_else(|| anyhow!("Expected at least one service"))?
                .collect();
            stop_service_cmd(modules_to_stop, cli_config)?;
        }
        ("down", Some(_down_cli_opts)) => {
            down_cmd(cli_config)?;
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
            let follow = logs_cli_opts.is_present("follow");
            let all = logs_cli_opts.is_present("all");

            let mode = if follow {
                LogMode::FOLLOW
            } else if all {
                LogMode::FULL
            } else {
                LogMode::DEFAULT
            };

            print_logs(module_name, mode, cli_config)?;
        }
        _ => {}
    }
    Ok(())
}
