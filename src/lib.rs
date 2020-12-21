#![allow(dead_code)]
#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use]
extern crate rocket;

#[macro_use]
extern crate simple_error;

pub mod client;
pub mod daemon;
pub mod dependency;
pub mod thread_control;
