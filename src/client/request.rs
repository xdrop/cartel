use crate::client::module::ServiceOrTaskDefinition;
use crate::daemon::api::*;
use anyhow::{bail, Result};
use reqwest::blocking::Client;
use std::time::Duration;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
#[serde(untagged)]
pub enum DeploymentResponse {
    Ok(ApiDeploymentResponse),
    Err(ErrorResponse),
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(untagged)]
pub enum TaskDeploymentResponse {
    Ok(ApiTaskDeploymentResponse),
    Err(ErrorResponse),
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(untagged)]
pub enum OperationResponse {
    Ok(ApiOperationResponse),
    Err(ErrorResponse),
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(untagged)]
pub enum LogInfoResponse {
    Ok(ApiLogResponse),
    Err(ErrorResponse),
}

fn long_timeout_client() -> Client {
    reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(180))
        .build()
        .unwrap()
}

pub fn deploy_module(
    module_definition: &ServiceOrTaskDefinition,
    force_deploy: bool,
    daemon_url: &str,
) -> Result<ApiDeploymentResponse> {
    let client = reqwest::blocking::Client::new();
    let command = ApiDeploymentCommand {
        module_definition: ApiModuleDefinition {
            name: module_definition.name.clone(),
            command: module_definition.command.clone(),
            environment: module_definition.environment.clone(),
            log_file_path: module_definition.log_file_path.clone(),
            dependencies: module_definition.dependencies.clone(),
            working_dir: module_definition.working_dir.clone(),
            termination_signal: (&module_definition.termination_signal).into(),
            healthcheck: module_definition
                .healthcheck
                .as_ref()
                .map(|h| h.into()),
        },
        force: force_deploy,
    };

    let deployment_result: DeploymentResponse = client
        .post(&(daemon_url.to_owned() + "/deploy"))
        .json(&command)
        .send()?
        .json()?;

    match deployment_result {
        DeploymentResponse::Ok(r) => Ok(r),
        DeploymentResponse::Err(e) => bail!(e.message),
    }
}

pub fn deploy_task(
    task_definition: &ServiceOrTaskDefinition,
    daemon_url: &str,
) -> Result<ApiTaskDeploymentResponse> {
    let client = long_timeout_client();
    let command = ApiTaskDeploymentCommand {
        task_definition: ApiModuleDefinition {
            name: task_definition.name.clone(),
            command: task_definition.command.clone(),
            environment: task_definition.environment.clone(),
            log_file_path: task_definition.log_file_path.clone(),
            dependencies: task_definition.dependencies.clone(),
            working_dir: task_definition.working_dir.clone(),
            termination_signal: ApiTermSignal::KILL,
            healthcheck: None,
        },
    };

    let deployment_result: TaskDeploymentResponse = client
        .post(&(daemon_url.to_owned() + "/tasks/deploy"))
        .json(&command)
        .send()?
        .json()?;

    match deployment_result {
        TaskDeploymentResponse::Ok(r) => Ok(r),
        TaskDeploymentResponse::Err(e) => bail!(e.message),
    }
}

pub fn stop_module(
    module_name: &str,
    daemon_url: &str,
) -> Result<ApiOperationResponse> {
    let client = reqwest::blocking::Client::new();
    let command = ApiOperationCommand {
        operation: ApiModuleOperation::STOP,
        module_name: module_name.to_string(),
    };

    let operation_result: OperationResponse = client
        .post(&(daemon_url.to_owned() + "/operation"))
        .json(&command)
        .send()?
        .json()?;

    match operation_result {
        OperationResponse::Ok(r) => Ok(r),
        OperationResponse::Err(e) => bail!(e.message),
    }
}

pub fn stop_all(daemon_url: &str) -> Result<ApiOperationResponse> {
    let client = reqwest::blocking::Client::new();

    let operation_result: OperationResponse = client
        .post(&(daemon_url.to_owned() + "/stop_all"))
        .send()?
        .json()?;

    match operation_result {
        OperationResponse::Ok(r) => Ok(r),
        OperationResponse::Err(e) => bail!(e.message),
    }
}

pub fn restart_module(
    module_name: &str,
    daemon_url: &str,
) -> Result<ApiOperationResponse> {
    let client = reqwest::blocking::Client::new();
    let command = ApiOperationCommand {
        operation: ApiModuleOperation::RESTART,
        module_name: module_name.to_string(),
    };

    let operation_result: OperationResponse = client
        .post(&(daemon_url.to_owned() + "/operation"))
        .json(&command)
        .send()?
        .json()?;

    match operation_result {
        OperationResponse::Ok(r) => Ok(r),
        OperationResponse::Err(e) => bail!(e.message),
    }
}

pub fn list_modules(daemon_url: &str) -> Result<ApiModuleStatusResponse> {
    let client = reqwest::blocking::Client::new();
    let status = client
        .get(&(daemon_url.to_owned() + "/status"))
        .send()?
        .json()?;

    Ok(status)
}

pub fn log_info(module_name: &str, daemon_url: &str) -> Result<ApiLogResponse> {
    let client = reqwest::blocking::Client::new();
    let status: LogInfoResponse = client
        .get(&(daemon_url.to_owned() + "/log/" + module_name))
        .send()?
        .json()?;

    match status {
        LogInfoResponse::Ok(r) => Ok(r),
        LogInfoResponse::Err(e) => bail!(e.message),
    }
}

pub fn poll_health(
    monitor_handle: &str,
    daemon_url: &str,
) -> Result<ApiHealthResponse> {
    let client = reqwest::blocking::Client::new();
    let health = client
        .get(&(daemon_url.to_owned() + "/health/" + monitor_handle))
        .send()?
        .json()?;

    Ok(health)
}
