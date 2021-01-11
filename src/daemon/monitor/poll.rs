use super::commands::*;
use super::state::{MonitorState, MonitorStatus};
use log::{debug, info};
use std::collections::HashMap;
use std::process::Stdio;
use std::sync::Arc;
use tokio::process;
use tokio::sync::mpsc;

pub(super) async fn poll_tickr(tx: mpsc::Sender<MonitorCommand>) {
    let mut interval =
        tokio::time::interval(tokio::time::Duration::from_secs(4));
    loop {
        interval.tick().await;
        tx.send(MonitorCommand::PerformPoll)
            .await
            .expect("Failed to transmit to monitor runtime");
    }
}

pub(super) async fn channel_rx(
    mut rx: mpsc::Receiver<MonitorCommand>,
    monitor_state: Arc<MonitorState>,
) {
    info!("Task spawned");
    let mut monitor_list: Vec<(String, Monitor)> = vec![];
    let mut attempt_count: HashMap<String, u32> = HashMap::new();

    while let Some(message) = rx.recv().await {
        match message {
            MonitorCommand::NewMonitor { key, monitor } => {
                info!("Registering monitor: {}", key);
                monitor_list.push((key, monitor));
            }
            MonitorCommand::PerformPoll => {
                let results =
                    perform_poll(&mut monitor_list, &mut attempt_count).await;
                monitor_state.update_states(results);
            }
        }
    }
}

async fn perform_poll(
    monitor_list: &mut Vec<(String, Monitor)>,
    attempt_count: &mut HashMap<String, u32>,
) -> Vec<(String, MonitorStatus)> {
    let poll_results = poll_monitors(&monitor_list).await;
    let mut status: Vec<(String, MonitorStatus)> = Vec::new();

    for (idx, (key, succesful)) in poll_results.into_iter().enumerate().rev() {
        let attempts = attempt_count.get(&key).copied().unwrap_or(0);
        if succesful {
            // If succeeded we want to remove it from the poll list
            monitor_list.swap_remove(idx);
            attempt_count.remove_entry(&key);
            status.push((key, MonitorStatus::Successful));
        } else if attempts > 6 {
            // If it failed too many times we also want to remove it
            monitor_list.swap_remove(idx);
            attempt_count.remove_entry(&key);
            status.push((key, MonitorStatus::RetriesExceeded));
        } else {
            // If it failed we want to track how many times it's failed
            *attempt_count.entry(key.to_string()).or_insert(0) += 1;
            status.push((key, MonitorStatus::Pending));
        }
    }
    status
}

async fn poll_monitors(monitors: &[(String, Monitor)]) -> Vec<(String, bool)> {
    let mut results = vec![];
    for (key, monitor) in monitors {
        match monitor {
            Monitor::Executable(exe_monitor) => {
                debug!("Polling exe monitor: {}", key);
                let result = poll_exe_monitor(&exe_monitor).await;
                debug!("Exe monitor result: {}", result);
                results.push((key.to_string(), result));
            }
        }
    }
    results
}

async fn poll_exe_monitor(exe_monitor: &ExeMonitor) -> bool {
    let (head, tail) = exe_monitor
        .command
        .split_first()
        .expect("Empty command in poll_exe_monitor");
    // TODO: Handle error
    let mut child = process::Command::new(head)
        .args(tail)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("failed to spawn");
    // TODO: Handle error
    let status = child.wait().await.expect("failed to await child");

    status.success()
}
