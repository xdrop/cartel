use crate::daemon::api::converter::*;
use crate::daemon::api::engine::CoreState;
use crate::daemon::api::error::*;
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
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ApiDeploymentCommand {
    pub to_deploy: Vec<String>,
    pub module_definitions: Vec<ApiModuleDefinition>,
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
    pub deployed: HashMap<String, bool>,
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

#[post("/api/v1/deploy", data = "<module>")]
pub fn deploy(
    module: Json<ApiDeploymentCommand>,
    core_state: State<CoreState>,
) -> ApiResult<ApiDeploymentResponse> {
    let module_inner = module.into_inner();
    let module_defs = module_inner
        .module_definitions
        .into_iter()
        .map(to_mod_def)
        .collect();

    let selection = module_inner.to_deploy;

    let deployed = core_state
        .core
        .planner()
        .deploy_many(module_defs, &selection)?;

    return Ok(Json(ApiDeploymentResponse {
        success: true,
        deployed,
    }));
}

#[post("/api/v1/operation", data = "<module>")]
pub fn module_operation(
    module: Json<ApiOperationCommand>,
    core_state: State<CoreState>,
) -> ApiResult<ApiOperationResponse> {
    let module = module.into_inner();
    let mut planner = core_state.core.planner();

    match module.operation {
        ApiModuleOperation::STOP => {
            planner.stop_module(&module.module_name);
        }
        _ => (),
    };
    Ok(Json(ApiOperationResponse { success: true }))
}

#[get("/api/v1/status")]
pub fn status(
    core_state: State<CoreState>,
) -> ApiResult<ApiModuleStatusResponse> {
    let status = core_state
        .core
        .planner()
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
pub fn log(
    module_name: String,
    core_state: State<CoreState>,
) -> ApiResult<ApiLogResponse> {
    let log_file_path = core_state
        .core
        .planner()
        .log_path(&module_name)?
        .to_os_string();

    Ok(Json(ApiLogResponse {
        log_file_path: log_file_path,
    }))
}

#[get("/")]
pub fn index() -> &'static str {
    "Hello, world!"
}
