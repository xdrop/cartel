use super::handlers;
use crate::daemon::Core;
use rocket::config::{Environment, LoggingLevel};
use rocket::Config;
use std::sync::Arc;

pub struct CoreState {
    pub core: Arc<Core>,
}

pub fn start(core: &Arc<Core>) {
    let cfg = Config::build(Environment::Production)
        .address("127.0.0.1")
        .port(13754)
        .log_level(LoggingLevel::Normal)
        .workers(4)
        .unwrap();

    rocket::custom(cfg)
        .manage(CoreState {
            core: Arc::clone(core),
        })
        .mount(
            "/",
            routes![
                handlers::index,
                handlers::health,
                handlers::deploy,
                handlers::deploy_task,
                handlers::status,
                handlers::stop_all,
                handlers::module_operation,
                handlers::log_file,
                handlers::get_plan
            ],
        )
        .launch();
}
