use crate::client::commands::DeployOptions;
use crate::client::module::{
    merge_env, InnerDefinition, ModuleDefinition, ModuleKind,
    ServiceOrTaskDefinition,
};
use crate::daemon::api::*;
use anyhow::{anyhow, bail, Result};
use core::convert::Into;
use reqwest::blocking::Client;
use std::collections::HashMap;
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

fn client(timeout: &Option<u64>) -> Client {
    reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(timeout.unwrap_or(180)))
        .build()
        .unwrap()
}

fn build_env_arg(
    svc: &ServiceOrTaskDefinition,
    opts: &DeployOptions,
) -> HashMap<String, String> {
    let mut base_env = svc.environment.clone();
    opts.active_envs.iter().for_each(|key| {
        if svc.environment_sets.contains_key(key) {
            let env_set = svc.environment_sets.get(key).unwrap();
            merge_env(&mut base_env, env_set);
        }
    });
    base_env
}

fn build_svc_module_definition(
    module_definition: &ServiceOrTaskDefinition,
    opts: &DeployOptions,
) -> ApiModuleDefinition {
    ApiModuleDefinition {
        kind: ApiModuleKind::Service,
        name: module_definition.name.clone(),
        command: module_definition.cmd_line(),
        environment: build_env_arg(module_definition, opts),
        log_file_path: module_definition.log_file_path.clone(),
        dependencies: module_definition.dependencies.clone(),
        working_dir: module_definition.working_dir.clone(),
        termination_signal: (&module_definition.termination_signal).into(),
        readiness_probe: module_definition
            .readiness_probe
            .as_ref()
            .map(Into::into),
        liveness_probe: module_definition
            .liveness_probe
            .as_ref()
            .map(Into::into),
    }
}

fn build_task_module_definition(
    task_definition: &ServiceOrTaskDefinition,
    opts: &DeployOptions,
) -> ApiModuleDefinition {
    ApiModuleDefinition {
        kind: ApiModuleKind::Task,
        name: task_definition.name.clone(),
        command: task_definition.cmd_line(),
        environment: build_env_arg(task_definition, opts),
        log_file_path: task_definition.log_file_path.clone(),
        dependencies: task_definition.dependencies.clone(),
        working_dir: task_definition.working_dir.clone(),
        termination_signal: ApiTermSignal::KILL,
        readiness_probe: None,
        liveness_probe: None,
    }
}

fn build_deploy_command(
    module_definition: &ServiceOrTaskDefinition,
    opts: &DeployOptions,
) -> ApiDeploymentCommand {
    ApiDeploymentCommand {
        module_definition: build_svc_module_definition(module_definition, opts),
        force: opts.force_deploy,
    }
}

fn build_task_deploy_command(
    task_definition: &ServiceOrTaskDefinition,
    opts: &DeployOptions,
) -> ApiTaskDeploymentCommand {
    ApiTaskDeploymentCommand {
        task_definition: build_task_module_definition(task_definition, opts),
    }
}

fn build_get_plan_request(
    modules: &[&ModuleDefinition],
    opts: &DeployOptions,
) -> ApiGetPlanRequest {
    let modules = modules
        .iter()
        .filter(|m| m.kind == ModuleKind::Service)
        .map(|m| match &m.inner {
            InnerDefinition::Service(svc) => {
                build_svc_module_definition(svc, opts)
            }
            _ => unreachable!(),
        })
        .collect();
    ApiGetPlanRequest { modules }
}

fn build_get_log_file_request(
    module_name: &str,
    module_kind: &ModuleKind,
) -> ApiLogFileRequest {
    ApiLogFileRequest {
        module_name: module_name.to_string(),
        module_kind: module_kind.into(),
    }
}

pub fn deploy_module(
    module_definition: &ServiceOrTaskDefinition,
    deploy_opts: &DeployOptions,
    daemon_url: &str,
) -> Result<ApiDeploymentResponse> {
    let client = reqwest::blocking::Client::new();
    let command = build_deploy_command(module_definition, deploy_opts);

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
    deploy_opts: &DeployOptions,
    daemon_url: &str,
) -> Result<ApiTaskDeploymentResponse> {
    let client = client(&task_definition.timeout);
    let command = build_task_deploy_command(task_definition, deploy_opts);

    let deployment_result: TaskDeploymentResponse = client
        .post(&(daemon_url.to_owned() + "/tasks/deploy"))
        .json(&command)
        .send()
        .map_err(|e| {
            if e.is_timeout() {
                anyhow!(task_took_too_long_msg(&task_definition.name))
            } else {
                e.into()
            }
        })?
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

pub fn log_file_path(
    module_name: &str,
    module_kind: &ModuleKind,
    daemon_url: &str,
) -> Result<ApiLogResponse> {
    let client = reqwest::blocking::Client::new();
    let request = build_get_log_file_request(module_name, module_kind);
    let status: LogInfoResponse = client
        .post(&(daemon_url.to_owned() + "/log_file"))
        .json(&request)
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

pub fn get_plan(
    modules: &[&ModuleDefinition],
    opts: &DeployOptions,
    daemon_url: &str,
) -> Result<ApiGetPlanResponse> {
    let client = reqwest::blocking::Client::new();
    let request = build_get_plan_request(modules, opts);
    let get_plan_result = client
        .post(&(daemon_url.to_owned() + "/get_plan"))
        .json(&request)
        .send()?
        .json()?;
    Ok(get_plan_result)
}

fn task_took_too_long_msg(task_name: &str) -> String {
    return format!(
        "Task \"{}\" took too long to finish. \
        \nTry increasing the timeout or check the \
        logs using `cartel logs {}`. \
        \n{} The task may still be running.",
        task_name,
        task_name,
        cbold!("Note:")
    );
}
