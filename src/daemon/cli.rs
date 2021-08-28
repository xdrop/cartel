use anyhow::Result;
use clap::{crate_version, App, Arg};

pub struct DaemonCliConfig {
    pub detach_tty: bool,
}

pub fn cli_app() -> Result<DaemonCliConfig> {
    let matches = App::new("cartel-daemon")
        .version(crate_version!())
        .about("Development workflow service orchestrator (daemon)")
        .arg(
            Arg::with_name("detach_tty")
                .short("d")
                .long("detach")
                .help("Detatches the daemon from its controlling terminal")
                .takes_value(false),
        )
        .get_matches();

    Ok(DaemonCliConfig {
        detach_tty: matches.is_present("detach_tty"),
    })
}
