use super::commands::{
    deploy_cmd, list_modules_cmd, print_logs, stop_module_cmd,
};
use clap::{App, Arg, ArgMatches, SubCommand};
use simple_error::SimpleError;
use std::env;

pub struct CliOptions {
    pub verbose: u64,
    pub pager_cmd: Vec<String>,
    pub daemon_url: String,
}

pub fn cli_app() {
    let matches = App::new("cartel")
        .version("0.1.0-alpha")
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
        .subcommand(
            SubCommand::with_name("deploy")
                .about("Deploys a service (and it's dependencies)")
                .arg(
                    Arg::with_name("modules")
                        .help("Modules to deploy")
                        .multiple(true)
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
                    Arg::with_name("module")
                        .help("The module to print the logs of")
                        .takes_value(true),
                ),
        )
        .subcommand(
            SubCommand::with_name("stop")
                .about("Stop a running service")
                .arg(
                    Arg::with_name("module")
                        .help("Module to stop")
                        .takes_value(true),
                ),
        )
        .get_matches();

    // TODO: Handle error
    let cli_config = cli_config(&matches).unwrap();
    // TODO: Handle error
    invoke_subcommand(&matches, &cli_config);
}

fn cli_config(matches: &ArgMatches) -> Result<CliOptions, SimpleError> {
    let pager_cmd = env::var("CARTEL_PAGER").unwrap_or("less +F".to_string());
    let pager_cmd: Vec<String> =
        pager_cmd.split(" ").map(|s| s.to_string()).collect();

    if pager_cmd.len() == 0 {
        bail!("Invalid log pager");
    }

    Ok(CliOptions {
        verbose: matches.occurrences_of("v"),
        pager_cmd,
        // TODO: Make config
        daemon_url: "http://localhost:8000/api/v1".to_string(),
    })
}

fn invoke_subcommand(matches: &ArgMatches, cli_config: &CliOptions) -> () {
    match matches.subcommand() {
        ("deploy", Some(deploy_cli_opts)) => {
            // TODO: Handle unwrap
            let modules_to_deploy =
                deploy_cli_opts.values_of("modules").unwrap().collect();
            deploy_cmd(modules_to_deploy, cli_config);
        }
        ("ps", Some(_)) => {
            list_modules_cmd(cli_config);
        }
        ("stop", Some(stop_cli_opts)) => {
            let module_to_stop = stop_cli_opts.value_of("module").unwrap();
            stop_module_cmd(module_to_stop, cli_config);
        }
        ("logs", Some(logs_cli_opts)) => {
            let module_name = logs_cli_opts.value_of("module").unwrap();
            print_logs(module_name, cli_config);
        }
        _ => {}
    }
}
