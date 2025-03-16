use crate::daemon::monitor::{self, MonitorHandle};
use crate::daemon::planner::Planner;
use crate::daemon::{api, env_grabber, signal};

use crate::config::{self, PersistedConfig};
use crate::daemon::env_grabber::{env_grabber_thread, CurrentEnvHolder};
use std::error::Error;
use std::sync::Arc;

/// Holds the core daemon state.
pub struct Core {
    pub planner: Planner,
    pub config: Arc<PersistedConfig>,
}

impl Core {
    // Note: We can potentially remove this wrapper struct if there isn't a
    // solid use case for having more than a planner in this. All interactions
    // can go directly to the planner.

    /// Initializes the daemon core.
    ///
    /// Most interaction with the daemon will happen through the [Planner]
    /// instance held in the core.
    pub fn new(
        monitor_handle: MonitorHandle,
        env_holder: Arc<CurrentEnvHolder>,
        cfg: Arc<PersistedConfig>,
    ) -> Core {
        Core {
            planner: Planner::new(monitor_handle, env_holder, Arc::clone(&cfg)),
            config: cfg,
        }
    }

    /// Return a reference to the planner.
    pub fn planner(&self) -> &Planner {
        &self.planner
    }

    /// Return a reference to the shared config.
    pub fn config(&self) -> &PersistedConfig {
        &self.config
    }
}

/// Start the daemon
#[rocket::main]
pub async fn start_daemon() -> Result<(), Box<dyn Error>> {
    let monitor = monitor::MonitorState::new();
    config::create_config_if_not_exists()?;
    let cfg = Arc::new(config::read_persisted_config()?);

    // Create the Tokio async runtime and pass a handle to it so that it can be
    // invoked from a sync context from within the API handlers.
    let monitor_handle = monitor::spawn_runtime(Arc::new(monitor));
    let env_holder = Arc::new(env_grabber::CurrentEnvHolder::new());
    let core = Arc::new(Core::new(
        monitor_handle,
        Arc::clone(&env_holder),
        Arc::clone(&cfg),
    ));

    // Setup signal handlers to collect dead child processes.
    signal::setup_signal_handlers(Arc::clone(&core))?;

    // Experimental: env-grabber thread (by default off)
    // This thread periodically starts a shell process in login + interactive
    // mode and collects the environment variables from this process. The
    // environment variables are persisted and then used when starting new
    // services or tasks. See [`env_grabber_thread`] for more.
    if cfg.daemon.use_env_grabber.unwrap_or(false) {
        env_grabber_thread(Arc::clone(&env_holder));
    }

    // Start the API.
    let engine = api::engine::build(&core);
    engine.launch().await?;
    Ok(())
}
