#![allow(dead_code)]

#[macro_use]
extern crate rocket;

#[macro_use]
pub mod client;
pub mod collections;
pub mod command_builder;
pub mod config;
pub mod constants;
pub mod daemon;
pub mod dependency;
pub mod detach;
pub mod path;
pub mod process;
pub mod shell;
pub mod thread_control;
