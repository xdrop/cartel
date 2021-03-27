use anyhow::Result;
use clap::{crate_version, App, Arg};

pub struct DaemonCliConfig {
    pub shell: Option<String>,
}

pub fn cli_app() -> Result<DaemonCliConfig> {
    let matches = App::new("cartel-daemon")
        .version(&crate_version!()[..])
        .about("Panayiotis P. <xdrop.me@gmail.com>")
        .about("Development workflow service orhchestrator (daemon)")
        .arg(
            Arg::with_name("shell")
                .short("s")
                .long("shell")
                .value_name("PATH")
                .help("Path to shell to start the daemon in")
                .takes_value(true),
        )
        .get_matches();

    Ok(DaemonCliConfig {
        shell: matches.value_of("shell").map(String::from),
    })
}
