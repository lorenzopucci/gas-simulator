use diesel::prelude::Queryable;
use rocket::http::Status;
use rocket::serde::json::Json;
use rocket_db_pools::Connection;
use serde::{Deserialize, Serialize};

use crate::api::ApiInputResult;
use crate::DB;


#[derive(Queryable, Serialize)]
pub struct JollyGetResponse {
    id: i32,
    team_id: i32,
    question_id: i32,
}

#[derive(Deserialize)]
pub struct JollyPostData {
    team_id: i32,
    question_id: i32,
}

#[derive(Deserialize)]
pub struct JollyPatchData {
    team_id: i32,
    question_id: i32,
}

#[get("/contests/<id>/jollies")]
pub async fn get_jollies<'r>(
    id: i32,
    mut db: Connection<DB>
) -> Status {
    Status::NotImplemented
}

#[post("/contests/<id>/jollies", format = "application/json", data = "<jolly>")]
pub async fn post_jolly<'r>(
    id: i32,
    jolly: ApiInputResult<'r, JollyPostData>,
    mut db: Connection<DB>
) -> Status {
    Status::NotImplemented
}

#[get("/contests/<id>/jollies/<jolly_id>")]
pub async fn get_jolly<'r>(
    id: i32,
    jolly_id: i32,
    mut db: Connection<DB>
) -> Status {
    Status::NotImplemented
}

#[patch("/contests/<id>/jollies/<jolly_id>", format = "application/json", data = "<data>")]
pub async fn patch_submission<'r>(
    id: i32,
    jolly_id: i32,
    data: Json<JollyPatchData>,
    mut db: Connection<DB>
) -> Status {
    Status::NotImplemented
}

#[delete("/contests/<id>/jollies/<jolly_id>")]
pub async fn delete_jolly<'r>(
    id: i32,
    jolly_id: i32,
    mut db: Connection<DB>
) -> Status {
    Status::NotImplemented
}

