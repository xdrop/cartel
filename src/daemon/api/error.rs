use anyhow::Error;
use rocket::http::Status;
use rocket::response::{Responder, Response};
use rocket_contrib::json::Json;
use serde::{Deserialize, Serialize};

#[derive(Debug)]
pub enum ApiError {
    DeploymentError(Error),
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ErrorResponse {
    pub status: String,
    pub message: String,
    pub code: u64,
}

pub type ApiResult<T> = Result<Json<T>, ApiError>;

impl<'r> Responder<'r> for ApiError {
    fn respond_to(
        self,
        req: &rocket::Request<'_>,
    ) -> Result<Response<'r>, Status> {
        let message = match self {
            ApiError::DeploymentError(error) => {
                if error.chain().len() > 1 {
                    format!("{}: {}", error.to_string(), error.root_cause())
                } else {
                    error.to_string()
                }
            }
        };
        Json(ErrorResponse {
            status: String::from("error"),
            message,
            code: 100,
        })
        .respond_to(req)
        .map(|mut res| {
            res.set_status(Status::BadRequest);
            res
        })
    }
}

impl From<Error> for ApiError {
    fn from(error: Error) -> ApiError {
        ApiError::DeploymentError(error)
    }
}
