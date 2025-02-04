use std::fmt::Display;

use diesel::prelude::Queryable;
use diesel::{ExpressionMethods, QueryDsl};
use rocket::data::FromData;
use rocket::http::{ContentType, HeaderMap, Status};
use rocket::request::{FromRequest, Outcome};
use rocket::response::{self, Responder};
use rocket::serde::json::Json;
use rocket::{Request, Response, Route};
use rocket_db_pools::diesel::prelude::RunQueryDsl;
use rocket_db_pools::Connection;
use serde::Serialize;
use tracing::warn;

use crate::DB;

mod contests;
mod login;

#[derive(Serialize, Queryable)]
pub struct ApiUser {
    pub user_id: i32,
    pub username: String,
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for ApiUser {
    type Error = &'r str;

    async fn from_request(req: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        use crate::schema::{users, tokens};

        let token = match req.cookies().get("api_key") {
            Some(token) => &token.to_string()[8..],
            None => return Outcome::Forward(Status::Unauthorized),
        };

        let Outcome::Success(mut db) = req.guard::<Connection<DB>>().await else {
            return Outcome::Error((Status::InternalServerError, "Errore nell'autenticazione"));
        };

        let user = match tokens::dsl::tokens
            .inner_join(users::table)
            .select((tokens::user_id, users::username))
            .filter(tokens::token.eq(token))
            .load::<ApiUser>(&mut db)
            .await
        {
            Ok(user) => user,
            Err(err) => {
                warn!("{}", err);
                return Outcome::Error((Status::InternalServerError, "Errore nell'autenticazione"));
            }
        };

        match user.into_iter().next() {
            Some(user) => Outcome::Success(user),
            None => Outcome::Forward(Status::Unauthorized),
        }
    }
}

#[derive(Clone, Debug)]
pub struct ApiResponse<'r, T> {
    pub status: Status,
    pub body: T,
    pub headers: HeaderMap<'r>,
}

#[derive(Clone, Debug, Serialize)]
pub struct ApiError {
    pub error: String,
}

impl<'r, 'o: 'r, T: serde::Serialize> Responder<'r, 'o> for ApiResponse<'o, T> {
    fn respond_to(self, req: &'r Request) -> response::Result<'o> {
        let mut resp = Response::build_from(Json(self.body).respond_to(req).unwrap());
        let mut resp = resp.status(self.status).header(ContentType::JSON);

        for header in self.headers.into_iter() {
            resp = resp.header(header)
        }

        resp.ok()
    }
}

fn prop_error(err: impl Display, status: Status, msg: &str) -> ApiResponse<ApiError> {
    warn!("{}", err);
    ApiResponse {
        status,
        body: ApiError { error: msg.to_string() },
        headers: HeaderMap::new(),
    }
}

type ApiInputResult<'r, T> = Result<Json<T>, <Json<T> as FromData<'r>>::Error>;

pub fn routes() -> Vec<Route> {
    routes![
        contests::get_contest,
        contests::post_contest,
        contests::patch_contest,
        contests::delete_contest,
        contests::teams::get_team,
        contests::teams::get_teams,
        contests::teams::post_team,
        contests::teams::delete_team,
        contests::questions::get_question,
        contests::questions::get_questions,
        contests::questions::post_question,
        contests::questions::delete_question,
        contests::submissions::get_submission,
        contests::submissions::get_submissions,
        contests::submissions::post_submission,
        contests::submissions::patch_submission,
        contests::submissions::delete_submission,
        contests::jollies::get_jolly,
        contests::jollies::get_jollies,
        contests::jollies::post_jolly,
        contests::jollies::delete_jolly,
        login::signup,
        login::login,
    ]
}
