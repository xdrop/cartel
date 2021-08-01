#![allow(dead_code)]
#![feature(proc_macro_hygiene, decl_macro)]

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
pub mod path;
pub mod process;
pub mod shell;
pub mod thread_control;
