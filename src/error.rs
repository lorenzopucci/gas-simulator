use std::fmt::{Debug, Display};
use std::result;

use rocket::http::Status;
use tracing::warn;

pub type Error = Status;
pub type Result<T> = result::Result<T, Error>;

pub trait IntoStatusResult<T> {
    fn attach_status(self, status: Status) -> Result<T>;
}

impl<T, U> IntoStatusResult<T> for result::Result<T, U>
where
    U: Display + Debug,
{
    fn attach_status(self, status: Status) -> Result<T> {
        self.map_err(|err| {
            warn!("\n{:?}", err);
            status
        })
    }
}
