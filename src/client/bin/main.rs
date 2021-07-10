extern crate cartel;

use cartel::client::cli::cli_app;
use cartel::{teprinterr, texiterr};

fn main() {
    if let Err(e) = cli_app() {
        texiterr!(e);
    }
}
