use super::commands::*;
use super::config::{read_persisted_config, PersistedConfig};
use anyhow::{anyhow, bail, Result};
use clap::{crate_version, App, AppSettings, Arg, ArgMatches, SubCommand};
use std::env;

pub struct ClientConfig {
    pub verbose: u64,
    pub module_file: Option<String>,
    pub override_file: Option<String>,
    pub default_pager_cmd: Vec<String>,
    pub full_pager_cmd: Vec<String>,
    pub follow_pager_cmd: Vec<String>,
    pub daemon_url: String,
    pub default_dir: Option<String>,
    pub skip_checks: bool,
}

pub fn cli_app() -> Result<()> {
    let matches = App::new("cartel")
        .version(&crate_version!()[..])
        .about("Panayiotis P. <xdrop.me@gmail.com>")
        .about("Development workflow service orhchestrator")
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
            Arg::with_name("override")
                .short("o")
                .long("override")
                .value_name("FILE")
                .help("Specify a module definitions file to override with")
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
                .short("z")
                .long("no-checks")
                .help("Disables running checks"),
        )
        .subcommand(
            SubCommand::with_name("deploy")
                .about("Deploys a module (and it's dependencies)")
                .visible_alias("d")
                .arg(
                    Arg::with_name("force")
                        .help("Force deploy all modules")
                        .short("f")
                        .long("force"),
                )
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

    let persisted_config = read_persisted_config()?;
    let cfg = cfg(&matches, persisted_config)?;
    invoke_subcommand(&matches, &cfg)?;
    Ok(())
}

fn cfg(
    matches: &ArgMatches,
    persisted_config: Option<PersistedConfig>,
) -> Result<ClientConfig> {
    let full_pager_cmd = parse_cmd_from_env("CARTEL_FULL_LOG_PAGER", "less")?;
    let default_pager_cmd =
        parse_cmd_from_env("CARTEL_DEFAULT_LOG_PAGER", "tail -f -n 30")?;
    let follow_pager_cmd =
        parse_cmd_from_env("CARTEL_FOLLOW_LOG_PAGER", "less +F")?;

    let daemon_url = match &persisted_config {
        Some(client_conf) => client_conf
            .daemon_port
            .map(|port| format!("http://localhost:{}/api/v1", port)),
        _ => None,
    };

    let default_dir =
        persisted_config.and_then(|mut cfg| cfg.default_dir.take());

    Ok(ClientConfig {
        verbose: matches.occurrences_of("v"),
        skip_checks: matches.is_present("skip_checks"),
        module_file: matches.value_of("file").map(String::from),
        override_file: matches.value_of("override").map(String::from),
        default_pager_cmd,
        full_pager_cmd,
        follow_pager_cmd,
        default_dir,
        daemon_url: daemon_url
            .unwrap_or_else(|| String::from("http://localhost:13754/api/v1")),
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

fn invoke_subcommand(matches: &ArgMatches, cfg: &ClientConfig) -> Result<()> {
    match matches.subcommand() {
        ("deploy", Some(deploy_cli_opts)) => {
            let modules_to_deploy = deploy_cli_opts
                .values_of("modules")
                .ok_or_else(|| anyhow!("Expected at least one module"))?
                .collect();
            let options = DeployOptions::from(deploy_cli_opts);
            deploy_cmd(modules_to_deploy, cfg, &options)?;
        }
        ("run", Some(run_cli_opts)) => {
            let task_name = run_cli_opts
                .value_of("task")
                .ok_or_else(|| anyhow!("Expected task name"))?;
            run_task_cmd(task_name, cfg)?;
        }
        ("ps", Some(_)) => {
            list_modules_cmd(cfg)?;
        }
        ("stop", Some(stop_cli_opts)) => {
            let modules_to_stop = stop_cli_opts
                .values_of("services")
                .ok_or_else(|| anyhow!("Expected at least one service"))?
                .collect();
            stop_service_cmd(modules_to_stop, cfg)?;
        }
        ("down", Some(_down_cli_opts)) => {
            down_cmd(cfg)?;
        }
        ("restart", Some(restart_cli_opts)) => {
            let module_to_restart = restart_cli_opts
                .value_of("service")
                .ok_or_else(|| anyhow!("Expected service name"))?;
            restart_module_cmd(module_to_restart, cfg)?;
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

            print_logs(module_name, mode, cfg)?;
        }
        _ => {}
    }
    Ok(())
}
