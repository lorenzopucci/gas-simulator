use rocket::data::FromData;
use rocket::http::{ContentType, HeaderMap, Status};
use rocket::response::{self, Responder};
use rocket::serde::json::Json;
use rocket::{Request, Response, Route};
use serde::Serialize;

mod contests;

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
    ]
}
