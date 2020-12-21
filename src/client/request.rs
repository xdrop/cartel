use super::super::daemon::api::*;
use super::cli::CliOptions;
use super::module::ModuleDefinitionV1;
use reqwest;

pub fn deploy_modules(
    services_to_deploy: &Vec<&str>,
    module_definitions: &Vec<ModuleDefinitionV1>,
    daemon_url: &String,
) -> Result<ApiDeploymentResponse, reqwest::Error> {
    let client = reqwest::blocking::Client::new();
    let command = ApiDeploymentCommand {
        to_deploy: services_to_deploy.iter().map(|s| s.to_string()).collect(),
        module_definitions: module_definitions
            .into_iter()
            .map(|m| ApiModuleDefinition {
                name: m.name.clone(),
                command: m.command.clone(),
                environment: m.environment.clone(),
                log_file_path: m.log_file_path.clone(),
                dependencies: m.dependencies.clone(),
            })
            .collect(),
    };

    let deployment_result = client
        .post(&(daemon_url.to_owned() + "/deploy"))
        .json(&command)
        .send()?
        .json()?;

    Ok(deployment_result)
}

pub fn stop_module(
    module_name: &str,
    daemon_url: &String,
) -> Result<ApiOperationResponse, reqwest::Error> {
    let client = reqwest::blocking::Client::new();
    let command = ApiOperationCommand {
        operation: ApiModuleOperation::STOP,
        module_name: module_name.to_string(),
    };

    let deployment_result = client
        .post(&(daemon_url.to_owned() + "/operation"))
        .json(&command)
        .send()?
        .json()?;

    Ok(deployment_result)
}

pub fn list_modules(
    daemon_url: &String,
) -> Result<ApiModuleStatusResponse, reqwest::Error> {
    let client = reqwest::blocking::Client::new();
    let status = client
        .get(&(daemon_url.to_owned() + "/status"))
        .send()?
        .json()?;

    Ok(status)
}

pub fn log_info(
    module_name: &str,
    daemon_url: &String,
) -> Result<ApiLogResponse, reqwest::Error> {
    let client = reqwest::blocking::Client::new();
    let status = client
        .get(&(daemon_url.to_owned() + "/log/" + module_name))
        .send()?
        .json()?;

    Ok(status)
}
