extern crate cartel;

use cartel::client::cli::cli_app;
use cartel::{teprint, texit};

fn main() {
    if let Err(e) = cli_app() {
        texit!(e);
    }
}
