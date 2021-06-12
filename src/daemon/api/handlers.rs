use crate::daemon::api::engine::CoreState;
use crate::daemon::api::error::*;
use crate::daemon::planner::{MonitorStatus, Planner};
use crate::daemon::{api::convert::*, monitor::MonitorType};
use rocket::State;
use rocket_contrib::json::Json;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::ffi::OsString;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ApiModuleKind {
    Task,
    Service,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ApiModuleDefinition {
    pub kind: ApiModuleKind,
    pub name: String,
    pub command: Vec<String>,
    pub environment: HashMap<String, String>,
    pub log_file_path: Option<String>,
    pub dependencies: Vec<String>,
    pub working_dir: Option<String>,
    pub termination_signal: ApiTermSignal,
    pub readiness_probe: Option<ApiProbe>,
    pub liveness_probe: Option<ApiProbe>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "kind")]
pub enum ApiProbe {
    Executable(ApiExeProbe),
    LogLine(ApiLogLineProbe),
    Net(ApiNetworkProbe),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ApiExeProbe {
    pub retries: u32,
    pub command: Vec<String>,
    pub working_dir: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ApiLogLineProbe {
    pub retries: u32,
    pub line_regex: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ApiNetworkProbe {
    pub retries: u32,
    pub hostname: String,
    pub port: u16,
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
    pub liveness_status: Option<ApiProbeStatus>,
    pub exit_code: Option<i32>,
    pub time_since_status: u64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ApiLogResponse {
    pub log_file_path: OsString,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "snake_case")]
pub enum ApiProbeStatus {
    Pending,
    Successful,
    RetriesExceeded,
    Failing,
    Error,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ApiHealthResponse {
    pub probe_status: Option<ApiProbeStatus>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "snake_case")]
pub enum ApiPlannedAction {
    WillRedeploy,
    WillDeploy,
    AlreadyDeployed,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ApiGetPlanRequest {
    pub modules: Vec<ApiModuleDefinition>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ApiGetPlanResponse {
    pub plan: HashMap<String, ApiPlannedAction>,
}

#[allow(clippy::unnecessary_unwrap)]
#[post("/api/v1/deploy", data = "<command>")]
pub(crate) fn deploy(
    command: Json<ApiDeploymentCommand>,
    core_state: State<CoreState>,
) -> ApiResult<ApiDeploymentResponse> {
    let planner = core_state.core.planner();
    let command = command.into_inner();

    let (module_def, monitor) =
        from_service_with_monitor(command.module_definition);
    let module_name = module_def.name.clone();

    let deployed = planner.deploy(module_def, command.force)?;

    let monitor_key = if deployed && monitor.is_some() {
        let monitor_key = planner.create_monitor(
            &module_name,
            monitor.unwrap(),
            MonitorType::Readiness,
        );
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
            liveness_status: match m.liveness_status {
                Some(ref s) => Some(s.into()),
                None => None,
            },
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

#[get("/api/v1/health/<monitor_key>")]
pub(crate) fn health(
    monitor_key: String,
    core_state: State<CoreState>,
) -> Json<ApiHealthResponse> {
    let status = core_state
        .core
        .planner()
        .monitor_status(monitor_key.as_str());

    let probe_status = match status {
        Some(MonitorStatus::Pending) => Some(ApiProbeStatus::Pending),
        Some(MonitorStatus::Successful) => Some(ApiProbeStatus::Successful),
        Some(MonitorStatus::RetriesExceeded) => {
            Some(ApiProbeStatus::RetriesExceeded)
        }
        Some(MonitorStatus::Error) => Some(ApiProbeStatus::Error),
        Some(MonitorStatus::Failing) => Some(ApiProbeStatus::Failing),
        None => None,
    };

    Json(ApiHealthResponse { probe_status })
}

#[post("/api/v1/get_plan", data = "<request>")]
pub(crate) fn get_plan(
    request: Json<ApiGetPlanRequest>,
    core_state: State<CoreState>,
) -> Json<ApiGetPlanResponse> {
    let planner = core_state.core.planner();
    let mut request = request.into_inner();

    let modules: Vec<_> = request
        .modules
        .drain(..)
        .map(from_task_or_service)
        .collect();

    let plan = planner.get_plan(&modules);
    Json(plan.into())
}

#[get("/")]
pub(crate) fn index() -> &'static str {
    "Daemon service"
}
