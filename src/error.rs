use std::result;

use rocket::http::Status;
use tracing::warn;

pub struct Error {
    error: anyhow::Error,
    status: Status,
}

pub type Result<T> = result::Result<T, Error>;

pub trait IntoStatusResult<T> {
    fn attach_status(self, status: Status) -> Result<T>;
}

impl<T, E> IntoStatusResult<T> for result::Result<T, E>
where
    E: Into<anyhow::Error>,
{
    fn attach_status(self, status: Status) -> Result<T> {
        self.map_err(|error| Error {
            error: error.into(),
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
        warn!("\n{}", value.error);
        value.status
    }
}
