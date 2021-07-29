use crate::client::commands::*;
use crate::config;
use crate::config::PersistedConfig;
use anyhow::{anyhow, bail, Error, Result};
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
}

pub fn cli_app() -> Result<()> {
    let matches = App::new("cartel")
        .version(crate_version!())
        .about("Development workflow service orchestrator")
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
                    Arg::with_name("env")
                        .short("e")
                        .long("env")
                        .help("Environment set to activate")
                        .takes_value(true)
                        .require_delimiter(true)
                        .value_delimiter("\0")
                        .multiple(true)
                        .long_help(
                            "Override the env of each service by \
                            activating environment sets \
                            with the given name. In case of overlaps, \
                            priority is given to the last defined.",
                        ),
                )
                .arg(
                    Arg::with_name("modules")
                        .help("Modules to deploy")
                        .multiple(true)
                        .takes_value(true),
                )
                .arg(
                    Arg::with_name("skip_checks")
                        .short("z")
                        .long("no-checks")
                        .help("Disables running checks"),
                )
                .arg(
                    Arg::with_name("only_selected")
                        .short("o")
                        .long("only-selected")
                        .help("Only deploy selected modules (no dependencies)"),
                )
                .arg(
                    Arg::with_name("wait")
                        .short("w")
                        .long("wait")
                        .conflicts_with("skip_readiness_checks")
                        .help("Waits for all readiness checks to complete"),
                )
                .arg(
                    Arg::with_name("serial")
                        .short("k")
                        .long("serial")
                        .help("Deploy one module at a time"),
                )
                .arg(
                    Arg::with_name("threads")
                        .short("t")
                        .long("threads")
                        .conflicts_with("serial")
                        .takes_value(true)
                        .help(
                            "Set the number of threads \
                            to use while deploying",
                        ),
                )
                .arg(
                    Arg::with_name("skip_readiness_checks")
                        .short("s")
                        .long("no-readiness")
                        .help("Disables running readiness checks"),
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
            SubCommand::with_name("shell")
                .about("Open a shell for the given service")
                .visible_alias("sh")
                .arg(
                    Arg::with_name("type")
                        .short("t")
                        .long("type")
                        .help("The shell type to open")
                        .takes_value(true),
                )
                .arg(
                    Arg::with_name("service")
                        .help("The service to open a shell for")
                        .takes_value(true),
                ),
        )
        .subcommand(
            SubCommand::with_name("ps")
                .about("Print currently running services")
                .arg(
                    Arg::with_name("no-color")
                        .short("n")
                        .long("no-color")
                        .help("Disable coloured output")
                        .takes_value(false),
                ),
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
        .subcommand(
            SubCommand::with_name("daemon")
                .about("Control the daemon")
                .subcommand(
                    SubCommand::with_name("restart")
                        .about("Restart the daemon"),
                ),
        )
        .subcommand(
            SubCommand::with_name("config")
                .about("Update configuration")
                .subcommand(
                    SubCommand::with_name("set")
                        .about("Sets a configuration option")
                        .arg(
                            Arg::with_name("key")
                                .help("The setting key to set")
                                .required(true)
                                .takes_value(true)
                                .multiple(false),
                        )
                        .arg(
                            Arg::with_name("value")
                                .help("The setting value to set")
                                .required(true)
                                .takes_value(true)
                                .multiple(false),
                        ),
                )
                .subcommand(
                    SubCommand::with_name("toggle")
                        .about(
                            "Toggles a (boolean) configuration option on/off",
                        )
                        .arg(
                            Arg::with_name("key")
                                .help("The setting to toggle")
                                .required(true)
                                .takes_value(true)
                                .multiple(false),
                        ),
                )
                .subcommand(
                    SubCommand::with_name("remove")
                        .about(
                            "Removes a configuration option (resetting to the default)",
                        )
                        .arg(
                            Arg::with_name("key")
                                .help("The setting to remove")
                                .required(true)
                                .takes_value(true)
                                .multiple(false),
                        ),
                )
                .subcommand(
                    SubCommand::with_name("get")
                        .about("Gets the value of a configuration option")
                        .arg(
                            Arg::with_name("key")
                                .help("The setting to get the value of")
                                .required(true)
                                .takes_value(true)
                                .multiple(false),
                        ),
                ),
        )
        .get_matches();

    config::create_config_if_not_exists()?;
    let persisted_config = config::read_persisted_config()?;
    let cfg = cfg(&matches, &persisted_config)?;
    invoke_subcommand(&matches, &cfg)
        .map_err(|e| handle_daemon_offline(e, cfg.verbose > 0))?;
    Ok(())
}

fn cfg(
    matches: &ArgMatches,
    persisted_config: &PersistedConfig,
) -> Result<ClientConfig> {
    let full_pager_cmd = parse_cmd_from_env("CARTEL_FULL_LOG_PAGER", "less")?;
    let default_pager_cmd =
        parse_cmd_from_env("CARTEL_DEFAULT_LOG_PAGER", "tail -f -n 30")?;
    let follow_pager_cmd =
        parse_cmd_from_env("CARTEL_FOLLOW_LOG_PAGER", "less +F")?;

    let daemon_url = persisted_config
        .daemon
        .port
        .as_ref()
        .map(|port| format!("http://localhost:{}/api/v1", port));

    let default_dir = persisted_config.client.default_dir.clone();

    Ok(ClientConfig {
        verbose: matches.occurrences_of("verbose"),
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
        ("ps", Some(ps_opts)) => {
            let opts = PsOpts::from(ps_opts);
            list_modules_cmd(&opts, cfg)?;
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
        ("shell", Some(shell_cli_opts)) => {
            let service_name = shell_cli_opts
                .value_of("service")
                .ok_or_else(|| anyhow!("Expected service name"))?;
            let shell_type = shell_cli_opts.value_of("type");
            open_shell(service_name, shell_type, cfg)?;
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
        ("daemon", _) => {
            restart_daemon()?;
        }
        ("config", Some(config_cli_opts)) => {
            match config_cli_opts.subcommand() {
                ("set", Some(opts)) => {
                    let key = opts.value_of("key").unwrap();
                    let value = opts.value_of("value").unwrap();
                    set_option(&key, &value)?;
                }
                ("toggle", Some(opts)) => {
                    let key = opts.value_of("key").unwrap();
                    toggle_option(key)?;
                }
                ("get", Some(opts)) => {
                    let key = opts.value_of("key").unwrap();
                    get_option(key)?;
                }
                ("remove", Some(opts)) => {
                    let key = opts.value_of("key").unwrap();
                    remove_option(key)?;
                }
                _ => {}
            }
        }
        _ => {}
    }
    Ok(())
}

fn handle_daemon_offline(err: Error, verbose: bool) -> Error {
    let is_conn_err =
        if let Some(req_err) = err.downcast_ref::<reqwest::Error>() {
            req_err.is_connect()
        } else {
            false
        };
    if is_conn_err {
        let msg = anyhow!(
            "Could not connect to daemon. \
                Is the daemon running? (try `cartel daemon restart`)",
        );

        if verbose {
            err.context(msg)
        } else {
            anyhow!(msg)
        }
    } else {
        err
    }
}
