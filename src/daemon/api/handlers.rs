use crate::daemon::api::convert::*;
use crate::daemon::api::engine::CoreState;
use crate::daemon::api::error::*;
use crate::daemon::planner::{Monitor, MonitorStatus, Planner};
use rocket::State;
use rocket_contrib::json::Json;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::ffi::OsString;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ApiModuleDefinition {
    pub name: String,
    pub command: Vec<String>,
    pub environment: HashMap<String, String>,
    pub log_file_path: Option<String>,
    pub dependencies: Vec<String>,
    pub working_dir: Option<String>,
    pub termination_signal: ApiTermSignal,
    pub healthcheck: Option<ApiHealthcheck>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "kind")]
pub enum ApiHealthcheck {
    Executable(ApiExeHealthcheck),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ApiExeHealthcheck {
    pub retries: u32,
    pub command: Vec<String>,
    pub working_dir: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ApiTermSignal {
    KILL,
    TERM,
    INT,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ApiDeploymentCommand {
    pub module_definition: ApiModuleDefinition,
    pub force: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ApiTaskDeploymentCommand {
    pub task_definition: ApiModuleDefinition,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ApiTaskDeploymentResponse {
    pub success: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ApiOperationCommand {
    pub module_name: String,
    pub operation: ApiModuleOperation,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ApiOperationResponse {
    pub success: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ApiDeploymentResponse {
    pub success: bool,
    pub deployed: bool,
    pub monitor: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ApiModuleStatusResponse {
    pub status: Vec<ApiModuleStatus>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub enum ApiModuleRunStatus {
    RUNNING,
    WAITING,
    STOPPED,
    EXITED,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum ApiModuleOperation {
    STOP,
    RESTART,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ApiKind {
    Task,
    Service,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ApiModuleStatus {
    pub name: String,
    pub pid: u32,
    pub status: ApiModuleRunStatus,
    pub exit_code: Option<i32>,
    pub time_since_status: u64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ApiLogResponse {
    pub log_file_path: OsString,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "snake_case")]
pub enum ApiHealthStatus {
    Pending,
    Successful,
    RetriesExceeded,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ApiHealthResponse {
    pub healthcheck_status: Option<ApiHealthStatus>,
}

#[allow(clippy::unnecessary_unwrap)]
#[post("/api/v1/deploy", data = "<command>")]
pub(crate) fn deploy(
    command: Json<ApiDeploymentCommand>,
    core_state: State<CoreState>,
) -> ApiResult<ApiDeploymentResponse> {
    let planner = core_state.core.planner();
    let command = command.into_inner();

    let monitor: Option<Monitor> = match command.module_definition.healthcheck {
        Some(ref h) => Some(h.into()),
        None => None,
    };

    let module_def = from_service(command.module_definition);
    let module_name = module_def.name.clone();

    let deployed = planner.deploy(module_def, command.force)?;

    let monitor_key = if deployed && monitor.is_some() {
        let monitor_key = planner.create_monitor(module_name, monitor.unwrap());
        Some(monitor_key)
    } else {
        None
    };

    Ok(Json(ApiDeploymentResponse {
        success: true,
        deployed,
        monitor: monitor_key,
    }))
}

#[post("/api/v1/tasks/deploy", data = "<task>")]
pub(crate) fn deploy_task(
    task: Json<ApiTaskDeploymentCommand>,
    _core_state: State<CoreState>,
) -> ApiResult<ApiTaskDeploymentResponse> {
    let cmd = task.into_inner();
    let task = cmd.task_definition;
    Planner::deploy_task(&from_task(task))?;
    Ok(Json(ApiTaskDeploymentResponse { success: true }))
}

#[post("/api/v1/operation", data = "<module>")]
pub(crate) fn module_operation(
    module: Json<ApiOperationCommand>,
    core_state: State<CoreState>,
) -> ApiResult<ApiOperationResponse> {
    let module = module.into_inner();
    let planner = core_state.core.planner();

    match module.operation {
        ApiModuleOperation::STOP => {
            planner.stop_module(&module.module_name)?;
        }
        ApiModuleOperation::RESTART => {
            planner.restart_module(&module.module_name)?;
        }
    };
    Ok(Json(ApiOperationResponse { success: true }))
}

#[post("/api/v1/stop_all")]
pub(crate) fn stop_all(
    core_state: State<CoreState>,
) -> ApiResult<ApiOperationResponse> {
    let planner = core_state.core.planner();
    planner.stop_all()?;

    Ok(Json(ApiOperationResponse { success: true }))
}

#[allow(clippy::unnecessary_wraps)]
#[get("/api/v1/status")]
pub(crate) fn status(
    core_state: State<CoreState>,
) -> ApiResult<ApiModuleStatusResponse> {
    let planner = core_state.core.planner();
    let status = planner
        .module_status()
        .into_iter()
        .map(|m| ApiModuleStatus {
            name: m.name,
            pid: m.pid,
            time_since_status: m.time_since_status,
            exit_code: m.exit_code,
            status: ApiModuleRunStatus::from(m.status),
        })
        .collect();

    Ok(Json(ApiModuleStatusResponse { status }))
}

#[get("/api/v1/log/<module_name>")]
pub(crate) fn log(
    module_name: String,
    core_state: State<CoreState>,
) -> ApiResult<ApiLogResponse> {
    let log_file_path = core_state.core.planner().log_path(&module_name)?;

    Ok(Json(ApiLogResponse { log_file_path }))
}

#[get("/api/v1/health/<module_name>")]
pub(crate) fn health(
    module_name: String,
    core_state: State<CoreState>,
) -> Json<ApiHealthResponse> {
    let status = core_state
        .core
        .planner()
        .monitor_status(module_name.as_str());

    let healthcheck_status = match status {
        Some(MonitorStatus::Pending) => Some(ApiHealthStatus::Pending),
        Some(MonitorStatus::Successful) => Some(ApiHealthStatus::Successful),
        Some(MonitorStatus::RetriesExceeded) => {
            Some(ApiHealthStatus::RetriesExceeded)
        }
        None => None,
    };

    Json(ApiHealthResponse { healthcheck_status })
}

#[get("/")]
pub(crate) fn index() -> &'static str {
    "Daemon service"
}
