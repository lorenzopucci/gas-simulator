use std::result;

use rocket::http::{HeaderMap, Status};
use tracing::error;

use crate::api::{ApiError, ApiResponse};

pub struct Error {
    error: anyhow::Error,
    message: String,
    status: Status,
}

pub type Result<T> = result::Result<T, Error>;

pub trait IntoStatusResult<T> {
    fn attach_info(self, status: Status, message: &str) -> Result<T>;
}

impl<T, E> IntoStatusResult<T> for result::Result<T, E>
where
    E: Into<anyhow::Error>,
{
    fn attach_info(self, status: Status, message: &str) -> Result<T> {
        self.map_err(|error| Error {
            error: error.into(),
            message: message.to_string(),
            status,
        })
    }
}

impl From<Error> for anyhow::Error {
    fn from(value: Error) -> Self {
        value.error
    }
}

impl From<Error> for Status {
    fn from(value: Error) -> Self {
        error!("\n{}", value.error);
        value.status
    }
}

impl From<Error> for ApiResponse<'_, ApiError> {
    fn from(value: Error) -> Self {
        error!("{}", value.error);
        Self {
            status: value.status,
            body: ApiError { error: value.message, },
            headers: HeaderMap::new(),
        }
    }
}
