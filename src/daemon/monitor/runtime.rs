use super::commands::*;
use super::poll::{channel_rx, poll_tickr};
use super::state::{MonitorState, MonitorStatus};
use anyhow::Result;
use log::info;
use std::sync::Arc;
use std::thread;
use tokio::runtime::{self, Handle, Runtime};
use tokio::sync::mpsc;

pub struct MonitorHandle {
    /// Handle to the tokio runtime of the monitor worker.
    runtime_handle: Handle,
    /// Producer side of a channel to publish commands to the monitor.
    producer: mpsc::Sender<MonitorCommand>,
    /// Reference to monitor instance containing the monitor state.
    monitor_state: Arc<MonitorState>,
}

impl MonitorHandle {
    fn from(
        runtime_handle: Handle,
        producer: mpsc::Sender<MonitorCommand>,
        monitor_state: Arc<MonitorState>,
    ) -> MonitorHandle {
        MonitorHandle {
            runtime_handle,
            producer,
            monitor_state,
        }
    }

    pub fn new_monitor(&self, name: String, monitor: Monitor) {
        let tx = self.producer.clone();
        self.runtime_handle.spawn(async move {
            let cmd = MonitorCommand::NewMonitor { key: name, monitor };
            tx.send(cmd)
                .await
                .expect("Failed to transmit to monitor runtime");
        });
    }

    pub fn monitor_status(&self, monitor_name: &str) -> Option<MonitorStatus> {
        self.monitor_state.monitor_status(monitor_name)
    }
}

fn setup_runtime() -> Result<Runtime, std::io::Error> {
    runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .thread_name("monitor-worker-thread")
        .enable_all()
        .build()
}

pub fn spawn_runtime(monitor_state: Arc<MonitorState>) -> MonitorHandle {
    let (tx, rx) = mpsc::channel::<MonitorCommand>(32);
    let tx_ = tx.clone();
    let (handle_tx, handle_rx) = std::sync::mpsc::channel();
    let mst = Arc::clone(&monitor_state);

    thread::spawn(move || {
        let runtime = setup_runtime().expect("Unable to create the runtime");
        info!("Runtime created");

        // Give a handle to the runtime to another thread
        handle_tx
            .send(runtime.handle().clone())
            .expect("Unable to give runtime handle to main thread");

        // Spawn the ticking thread for scanning of monitors
        runtime.spawn(async move { poll_tickr(tx_).await });

        // Continue running until notified to shutdown
        runtime.block_on(async { channel_rx(rx, mst).await });

        info!("Runtime finished");
    });

    let handle = handle_rx
        .recv()
        .expect("Could not get a handle to the tokio runtime");

    MonitorHandle::from(handle, tx, monitor_state)
}
