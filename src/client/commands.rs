use super::cli::CliOptions;
use super::config::read_module_definitions;
use super::progress::WaitSpin;
use super::request::*;
use crate::daemon::api::ApiModuleRunStatus;
use crate::dependency::DependencyGraph;
use anyhow::Result;
use chrono::Local;
use console::{style, Emoji};
use std::convert::TryFrom;
use std::process::Command;
use std::time::Duration;

static LOOKING_GLASS: Emoji<'_, '_> = Emoji("🔍  ", "");
static HOUR_GLASS: Emoji<'_, '_> = Emoji("⏳  ", "");
static UP_ARROW: Emoji<'_, '_> = Emoji("⬆️  ", "");
static SUCCESS: Emoji<'_, '_> = Emoji("✅  ", "");

pub fn deploy_cmd(
    modules_to_deploy: Vec<&str>,
    cli_config: &CliOptions,
) -> Result<()> {
    println!(
        "{} {}Looking for module definitions...",
        style("[1/4]").bold().dim(),
        LOOKING_GLASS
    );
    // TODO: Handle expect
    let module_defs = read_module_definitions()?;

    println!(
        "{} {} Resolving dependencies...",
        style("[2/4]").bold().dim(),
        UP_ARROW
    );
    let dependency_graph =
        DependencyGraph::from(&module_defs, &modules_to_deploy);
    let ordered = dependency_graph.dependency_sort()?;

    println!("{} {}Deploying...", style("[3/4]").bold().dim(), HOUR_GLASS);

    &ordered.iter().for_each(|m| {
        let mut ws = WaitSpin::new();
        ws.start(3, 4, format!("  Deploying: {}", m.name));
        // TODO: handle error
        deploy_modules(&vec![&m.name], &module_defs, &cli_config.daemon_url);
        ws.stop();
    });

    println!(
        "{} {}{}: {:?}",
        style("[4/4]").bold().dim(),
        SUCCESS,
        style("Deployed modules").bold().green(),
        &ordered
            .iter()
            .map(|m| m.name.clone())
            .collect::<Vec<String>>()
    );
    Ok(())
}

pub fn stop_module_cmd(module: &str, cli_config: &CliOptions) -> () {
    println!(
        "{} {}Stopping module...",
        style("[1/2]").bold().dim(),
        HOUR_GLASS
    );
    stop_module(module, &cli_config.daemon_url);

    println!(
        "{} {}{}: {:?}",
        style("[2/2]").bold().dim(),
        SUCCESS,
        style("Module stopped").bold().green(),
        &module
    );
}

pub fn list_modules_cmd(cli_config: &CliOptions) -> () {
    let module_status = list_modules(&cli_config.daemon_url);

    if let Ok(module_status) = module_status {
        println!("{:<8}{:<12}{:<12}{:<8}", "pid", "name", "status", "since");
        module_status.status.iter().for_each(|mod_status| {
            let formatted_status = match mod_status.status {
                ApiModuleRunStatus::RUNNING => "running",
                ApiModuleRunStatus::STOPPED => "stopped",
                ApiModuleRunStatus::WAITING => "waiting",
                ApiModuleRunStatus::EXITED => "exited",
            };
            let time_formatter = timeago::Formatter::new();
            let now = u64::try_from(Local::now().timestamp()).unwrap();
            let dur = Duration::new(now - mod_status.time_since_status, 0);

            println!(
                "{:<8}{:<12}{:<12}{:<8}",
                mod_status.pid,
                mod_status.name,
                formatted_status,
                time_formatter.convert(dur)
            );
        })
    }
}

pub fn print_logs(module_name: &str, cli_config: &CliOptions) {
    // TODO: Error handling
    let log_file = log_info(module_name, &cli_config.daemon_url).unwrap();

    // This might fail on systems like Windows since paths may not be UTF-8
    // encoded there. Since we are using 'less' to page the logs and we don't
    // support Windows this is not currently an issue, but worth revisiting
    // if support for Windows is to be added.
    let unix_path = log_file
        .log_file_path
        .to_str()
        .expect("Systems where paths aren't UTF-8 encoded are not supported");

    Command::new(&cli_config.pager_cmd[0])
        .args(&cli_config.pager_cmd[1..])
        .arg(unix_path)
        .spawn()
        .expect("")
        .wait();
}
