use std::result;
use std::fmt::{Debug, Display};

use rocket::http::Status;
use tracing::error;

pub trait Errorable: Debug + Display { }

impl<T: Debug + Display> Errorable for T { }

#[derive(Debug)]
pub struct Error<'a> {
    error: Box<dyn Errorable + 'a>,
    status: Status,
}

impl From<Error<'_>> for Status {
    fn from(value: Error) -> Status {
        error!("\n{:?}", value.error);
        value.status
    }
}

pub type Result<'a, T> = result::Result<T, Error<'a>>;

pub trait IntoStatusError<'a>
where
    Self: Errorable + Sized + 'a
{
    fn attach_status(self, status: Status) -> Error<'a> {
        Error {
            error: Box::new(self),
            status,
        }
    }
}


impl<'a, T> IntoStatusError<'a> for T
where
    T: Errorable + Sized + 'a
{ }

pub trait IntoStatusResult<'a, T> {
    fn attach_status(self, status: Status) -> Result<'a, T>;
}

impl<'a, T, U> IntoStatusResult<'a, T> for result::Result<T, U>
where
    U: Errorable + Sized + 'a
{
    fn attach_status(self, status: Status) -> Result<'a, T> {
        self.map_err(|err| err.attach_status(status))
    }
}

impl<'a, T> From<T> for Error<'a>
where
    T: Errorable + Sized + 'a
{
    fn from(value: T) -> Self {
        value.attach_status(Status::InternalServerError)
    }
}
