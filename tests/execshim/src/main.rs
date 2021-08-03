use signal_hook::consts::{SIGINT, SIGTERM};
use signal_hook::iterator::Signals;
use signal_hook::low_level;
use std::env;
use std::fs::OpenOptions;
use std::io::prelude::*;
use std::io::{Result, SeekFrom};
use std::os::raw::c_int;
use std::process::exit;

// This is a simple executable used as a shim to simulate and test interaction
// with various kinds of processes.

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() <= 1 {
        eprintln!("Not enough arguments");
        exit(1);
    }

    let switch = &args[1];
    match switch.as_str() {
        "blocked" => {
            if args.len() < 4 {
                eprintln!("Not enough arguments. Usage execshim blocked <path> <message>");
                exit(1);
            }
            let path = &args[2];
            let message = &args[3];
            if let Err(e) = update_counter(path) {
                eprintln!("Failed to increment counter. Error: {}", e);
                exit(1);
            }
            println!("{}", message);
            if let Err(e) = wait_for_signal(path) {
                eprintln!("Failed to update signal. Error: {}", e);
                exit(1);
            }
        }
        "unblocked" => {
            if args.len() < 4 {
                eprintln!("Not enough arguments. Usage execshim unblocked <path> <message>");
                exit(1);
            }

            let path = &args[2];
            let message = &args[3];

            if let Err(e) = update_counter(path) {
                eprintln!("Failed to increment counter. Error: {}", e);
                exit(1);
            }
            println!("{}", message);
        }
        _ => {
            eprintln!("Invalid argument")
        }
    }
}

fn wait_for_signal(path: &str) -> Result<()> {
    const SIGNALS: &[c_int] = &[SIGTERM, SIGINT];
    let mut signals = Signals::new(SIGNALS).unwrap();
    for signal in &mut signals {
        if signal == SIGTERM {
            println!("SIGTERM");
            update_signal(path, "SIGTERM")?;
        } else if signal == SIGINT {
            println!("SIGINT");
            update_signal(path, "SIGINT")?;
        }
        low_level::emulate_default_handler(signal).unwrap();
    }
    Ok(())
}

fn update_counter(path: &str) -> Result<()> {
    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(path)?;

    let mut contents = String::new();
    file.read_to_string(&mut contents)?;

    if contents == "" {
        contents = String::from("0|None")
    };

    let (counter, signal) =
        contents.split_once("|").expect("Unexpected file contents");
    let mut counter = counter.parse::<i32>().expect("Unexpected file contents");

    counter += 1;

    let new_content = format!("{}|{}", counter, signal);

    file.set_len(0)?;
    file.seek(SeekFrom::Start(0))?;
    file.write_all(new_content.as_bytes())?;

    Ok(())
}

fn update_signal(path: &str, new_signal: &str) -> Result<()> {
    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(path)?;

    let mut contents = String::new();
    file.read_to_string(&mut contents)?;

    let (counter, _) =
        contents.split_once("|").expect("Unexpected file contents");

    let new_content = format!("{}|{}", counter, new_signal);

    file.set_len(0)?;
    file.seek(SeekFrom::Start(0))?;
    file.write_all(new_content.as_bytes())?;

    Ok(())
}
