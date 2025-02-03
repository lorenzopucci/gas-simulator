use diesel::prelude::Queryable;
use rocket::http::Status;
use rocket::serde::json::Json;
use rocket_db_pools::Connection;
use serde::{Deserialize, Serialize};

use crate::api::ApiInputResult;
use crate::DB;

#[derive(Queryable, Serialize)]
pub struct SubmissionGetResponse {
    id: i32,
    answer: i32,
    team_id: i32,
    question_id: i32,
}

#[derive(Deserialize)]
pub struct SubmissionPostData {
    answer: i32,
    team_id: i32,
    question_id: i32,
}

#[derive(Deserialize)]
pub struct SubmissionPatchData {
    answer: i32,
    team_id: i32,
    question_id: i32,
}

#[get("/contests/<id>/submissions")]
pub async fn get_submissions(
    id: i32,
    mut db: Connection<DB>
) -> Status {
    Status::NotImplemented
}

#[post("/contests/<id>/submissions", format = "application/json", data = "<submission>")]
pub async fn post_submission(
    id: i32,
    submission: ApiInputResult<'_, SubmissionPostData>,
    mut db: Connection<DB>
) -> Status {
    Status::NotImplemented
}

#[get("/contests/<id>/submissions/<submission_id>")]
pub async fn get_submission(
    id: i32,
    submission_id: i32,
    mut db: Connection<DB>
) -> Status {
    Status::NotImplemented
}

#[patch("/contests/<id>/submissions/<submission_id>", format = "application/json", data = "<data>")]
pub async fn patch_submission(
    id: i32,
    submission_id: i32,
    data: Json<SubmissionPatchData>,
    mut db: Connection<DB>
) -> Status {
    Status::NotImplemented
}

#[delete("/contests/<id>/submissions/<submission_id>")]
pub async fn delete_submission(
    id: i32,
    submission_id: i32,
    mut db: Connection<DB>
) -> Status {
    Status::NotImplemented
}

