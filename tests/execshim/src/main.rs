use signal_hook::consts::{SIGINT, SIGTERM};
use signal_hook::iterator::Signals;
use signal_hook::low_level;
use std::fs::OpenOptions;
use std::io::prelude::*;
use std::io::{Read, Result, SeekFrom, Write};
use std::net::{TcpListener, TcpStream};
use std::os::raw::c_int;
use std::process::exit;
use std::time::{SystemTime, UNIX_EPOCH};
use std::{env, thread};

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
        "http" => {
            if args.len() < 3 {
                eprintln!("Not enough arguments. Usage execshim http <port>");
                exit(1);
            }
            let port = &args[2];
            http_listener(port);
        }
        "eventual_exit" => {
            if args.len() < 4 {
                eprintln!(
                    "Not enough arguments. Usage execshim eventual_exit <path> <delay>"
                );
                exit(1);
            }
            let path = &args[2];
            let delay = &args[3];
            if let Err(e) = eventual_exit(path, delay) {
                eprintln!("Error: {}", e);
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

fn handle_read(mut stream: &TcpStream) {
    let mut buf = [0u8; 4096];
    match stream.read(&mut buf) {
        Ok(_) => {
            let req_str = String::from_utf8_lossy(&buf);
            println!("{}", req_str);
        }
        Err(e) => println!("Unable to read stream: {}", e),
    }
}

fn handle_write(mut stream: TcpStream) {
    let response = b"HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=UTF-8\r\n\r\n<html><body>Hello world</body></html>\r\n";
    match stream.write(response) {
        Ok(_) => println!("Response sent"),
        Err(e) => println!("Failed sending response: {}", e),
    }
}

fn handle_client(stream: TcpStream) {
    handle_read(&stream);
    handle_write(stream);
}

fn http_listener(port: &str) {
    let listener = TcpListener::bind(format!("127.0.0.1:{}", port)).unwrap();
    println!("Listening for connections on port {}", port);

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                thread::spawn(|| handle_client(stream));
            }
            Err(e) => {
                println!("Unable to connect: {}", e);
            }
        }
    }
}

fn eventual_exit(path: &str, delay: &str) -> Result<()> {
    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(path)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards")
        .as_secs();
    if let Ok(t) = contents.parse::<u64>() {
        let seconds = delay.parse::<u64>().expect("Invalid delay");
        if t + seconds < now {
            exit(0);
        } else {
            exit(1);
        }
    } else {
        file.set_len(0)?;
        file.seek(SeekFrom::Start(0))?;
        file.write_all(now.to_string().as_bytes())?;
        exit(1);
    };
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
