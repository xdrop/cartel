use anyhow::Result;
use clap::{crate_version, App, Arg};

pub struct DaemonCliConfig {
    pub shell: Option<String>,
    pub detach_tty: bool,
}

pub fn cli_app() -> Result<DaemonCliConfig> {
    let matches = App::new("cartel-daemon")
        .version(crate_version!())
        .about("Development workflow service orchestrator (daemon)")
        .arg(
            Arg::with_name("shell")
                .short("s")
                .long("shell")
                .value_name("PATH")
                .help("Path to shell to start the daemon in")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("detach_tty")
                .short("d")
                .long("detach")
                .help("Detatches the daemon from its controlling terminal")
                .takes_value(false),
        )
        .get_matches();

    Ok(DaemonCliConfig {
        shell: matches.value_of("shell").map(String::from),
        detach_tty: matches.is_present("detach_tty"),
    })
}
