use crate::daemon::monitor::commands::*;
use crate::daemon::monitor::state::{MonitorState, MonitorStatus};
use crate::daemon::time::epoch_now;
use anyhow::{anyhow, Result};
use grep_matcher::Matcher;
use grep_regex::RegexMatcher;
use grep_searcher::sinks::UTF8;
use grep_searcher::Searcher;
use log::{debug, info};
use std::collections::HashMap;
use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::process;
use tokio::sync::mpsc;
use tokio::time::timeout;

pub(super) async fn readiness_poll_tickr(tx: mpsc::Sender<MonitorCommand>) {
    let mut interval =
        tokio::time::interval(tokio::time::Duration::from_secs(4));
    loop {
        interval.tick().await;
        tx.send(MonitorCommand::PollReadinessCheck)
            .await
            .expect("Failed to transmit to monitor runtime");
    }
}

pub(super) async fn liveness_poll_tickr(tx: mpsc::Sender<MonitorCommand>) {
    let mut interval =
        tokio::time::interval(tokio::time::Duration::from_secs(5));
    loop {
        interval.tick().await;
        tx.send(MonitorCommand::PollLivenessCheck)
            .await
            .expect("Failed to transmit to monitor runtime");
    }
}

pub(super) async fn cleanup_tickr(tx: mpsc::Sender<MonitorCommand>) {
    let mut interval =
        tokio::time::interval(tokio::time::Duration::from_secs(300));
    // Skip the first tick as it is instant
    interval.tick().await;

    loop {
        interval.tick().await;
        tx.send(MonitorCommand::CleanupIdleMonitors)
            .await
            .expect("Failed to transmit to monitor runtime");
    }
}

pub(super) async fn channel_rx(
    mut rx: mpsc::Receiver<MonitorCommand>,
    monitor_state: Arc<MonitorState>,
) {
    info!("Task spawned");
    let mut readiness_monitor_list: Vec<(String, Monitor)> = vec![];
    let mut readiness_admission_times: Vec<(String, u64)> = vec![];
    let mut liveness_monitor_list: Vec<(String, Monitor)> = vec![];
    let mut attempt_count: HashMap<String, u32> = HashMap::new();

    while let Some(message) = rx.recv().await {
        match message {
            MonitorCommand::NewMonitor {
                key,
                monitor,
                monitor_type,
            } => {
                info!("Registering monitor: {}", key);
                match monitor_type {
                    MonitorType::Liveness => {
                        liveness_monitor_list.push((key, monitor))
                    }
                    MonitorType::Readiness => {
                        readiness_monitor_list.push((key.clone(), monitor));
                        readiness_admission_times.push((key, epoch_now()));
                    }
                };
            }
            MonitorCommand::RemoveMonitor { key, monitor_type } => {
                info!("Removing monitor: {}", key);
                match monitor_type {
                    MonitorType::Liveness => {
                        if let Some(index) = liveness_monitor_list
                            .iter()
                            .position(|(k, _)| key == *k)
                        {
                            liveness_monitor_list.swap_remove(index);
                        }
                    }
                    MonitorType::Readiness => {
                        if let Some(index) = readiness_monitor_list
                            .iter()
                            .position(|(k, _)| key == *k)
                        {
                            readiness_monitor_list.swap_remove(index);
                        }
                    }
                };
            }
            MonitorCommand::PollReadinessCheck => {
                let results = poll_readiness_check(
                    &mut readiness_monitor_list,
                    &mut attempt_count,
                )
                .await;
                monitor_state.update_states(results);
            }
            MonitorCommand::PollLivenessCheck => {
                let results =
                    poll_liveness_check(&mut liveness_monitor_list).await;
                monitor_state.update_states(results);
            }
            MonitorCommand::CleanupIdleMonitors => {
                info!("Cleaning up idle monitors...");
                let now = epoch_now();

                // Collect expired indices
                let expired: Vec<usize> = readiness_admission_times
                    .iter()
                    .enumerate()
                    .rev()
                    .filter_map(|(idx, (_, admission_time))| {
                        let duration = Duration::new(now - admission_time, 0);

                        // Remove if admitted more than 10mins ago
                        if duration.as_secs() > 600 {
                            Some(idx)
                        } else {
                            None
                        }
                    })
                    .collect();

                // Map them to monitor keys
                let keys: Vec<_> = expired
                    .iter()
                    .map(|idx| readiness_admission_times[*idx].0.as_str())
                    .collect();

                debug!("For removal: {:?}", keys);
                monitor_state.remove_keys(&keys);

                // Remove from admission list
                expired.iter().for_each(|idx| {
                    readiness_admission_times.swap_remove(*idx);
                });
            }
        }
    }
}

async fn poll_readiness_check(
    monitor_list: &mut Vec<(String, Monitor)>,
    attempt_count: &mut HashMap<String, u32>,
) -> Vec<(String, MonitorStatus)> {
    let poll_results = poll_monitors(&monitor_list).await;
    let mut status: Vec<(String, MonitorStatus)> = Vec::new();

    for (idx, (key, result)) in poll_results.into_iter().enumerate().rev() {
        let retries = monitor_list[idx].1.retries;
        let attempts = *attempt_count.entry(key.to_string()).or_insert(1);
        let is_error = result.is_err();
        let poll_successful = result.unwrap_or(false);

        if is_error {
            // If the poll errored remove and set status to error
            monitor_list.swap_remove(idx);
            attempt_count.remove_entry(&key);
            status.push((key, MonitorStatus::Error));
        } else if poll_successful {
            // If the poll succeeded remove and set status to error
            monitor_list.swap_remove(idx);
            attempt_count.remove_entry(&key);
            status.push((key, MonitorStatus::Successful));
        } else if attempts >= retries {
            // If it failed too many times remove and update status
            monitor_list.swap_remove(idx);
            attempt_count.remove_entry(&key);
            status.push((key, MonitorStatus::RetriesExceeded));
        } else {
            // If it failed we want to track how many times it's failed
            *attempt_count.get_mut(&key).unwrap() += 1;
            status.push((key, MonitorStatus::Pending));
        }
    }
    status
}

async fn poll_liveness_check(
    monitor_list: &mut Vec<(String, Monitor)>,
) -> Vec<(String, MonitorStatus)> {
    let poll_results = poll_monitors(&monitor_list).await;
    let mut status: Vec<(String, MonitorStatus)> = Vec::new();

    for (idx, (key, result)) in poll_results.into_iter().enumerate().rev() {
        let is_error = result.is_err();
        let poll_successful = result.unwrap_or(false);

        if is_error {
            // If the poll errored remove and set status to error
            monitor_list.swap_remove(idx);
            status.push((key, MonitorStatus::Error));
        } else if !poll_successful {
            // If the poll failed set its status to failing
            status.push((key, MonitorStatus::Failing));
        } else {
            // Otherwise set it as successful
            status.push((key, MonitorStatus::Successful));
        }
    }
    status
}

async fn poll_monitors(
    monitors: &[(String, Monitor)],
) -> Vec<(String, Result<bool>)> {
    let mut results = vec![];
    for (key, monitor) in monitors {
        match &monitor.task {
            MonitorTask::Executable(exe_monitor) => {
                debug!("Polling exe monitor: {}", key);
                let result = poll_exe_monitor(&exe_monitor).await;
                debug!("Exe monitor result: {:?}", result);
                results.push((key.to_string(), result));
            }
            MonitorTask::LogLine(log_line_monitor) => {
                debug!("Polling log line monitor: {}", key);
                let result = poll_log_line_monitor(&log_line_monitor).await;
                debug!("Log line monitor result: {:?}", result);
                results.push((key.to_string(), result));
            }
            MonitorTask::Net(net_monitor) => {
                debug!("Polling net monitor: {}", key);
                let result = poll_net_monitor(&net_monitor).await;
                debug!("Net monitor result: {:?}", result);
                results.push((key.to_string(), result));
            }
        }
    }
    results
}

async fn poll_exe_monitor(exe_monitor: &ExecMonitor) -> Result<bool> {
    let (head, tail) = exe_monitor
        .command
        .split_first()
        .ok_or_else(|| anyhow!("Empty command in exe monitor"))?;

    let mut child = process::Command::new(head)
        .args(tail)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;

    let status = child.wait().await?;
    Ok(status.success())
}

async fn poll_log_line_monitor(
    log_line_monitor: &LogLineMonitor,
) -> Result<bool> {
    // TODO: Share the Searcher / RegexMatcher if expensive
    let matcher =
        RegexMatcher::new(&log_line_monitor.line_regex).expect("Invalid regex");
    let mut found = false;

    Searcher::new().search_path(
        &matcher,
        log_line_monitor.file_path.as_path(),
        UTF8(|_, line| {
            let match_ = matcher.find(line.as_bytes())?;
            if match_.is_some() {
                found = true;
                Ok(false)
            } else {
                Ok(true)
            }
        }),
    )?;

    Ok(found)
}

async fn poll_net_monitor(net_monitor: &NetMonitor) -> Result<bool> {
    let conn_fut = TcpStream::connect(format!(
        "{}:{}",
        net_monitor.hostname, net_monitor.port
    ));

    let result = match timeout(Duration::from_millis(100), conn_fut).await {
        Ok(future) => future.is_ok(),
        Err(_) => false,
    };

    Ok(result)
}
