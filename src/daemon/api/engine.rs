use super::handlers;
use crate::daemon::Core;
use std::sync::Arc;

pub struct CoreState {
    pub core: Arc<Core>,
}

pub fn start(core: &Arc<Core>) {
    rocket::ignite()
        .manage(CoreState {
            core: Arc::clone(core),
        })
        .mount(
            "/",
            routes![
                handlers::index,
                handlers::deploy,
                handlers::deploy_task,
                handlers::status,
                handlers::module_operation,
                handlers::log
            ],
        )
        .launch();
}
