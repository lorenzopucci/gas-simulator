use anyhow::anyhow;
use diesel::{ExpressionMethods, QueryDsl};
use rocket::http::Status;
use rocket::response::Redirect;
use rocket::Route;
use rocket_db_pools::diesel::prelude::RunQueryDsl;
use rocket_db_pools::Connection;
use rocket_dyn_templates::context;
use rocket_dyn_templates::Template;

use super::fetch::{fetch_contest, fetch_contest_with_ranking};
use crate::error::IntoStatusResult;
use crate::{model, DB};

#[get("/create")]
async fn create_redirect() -> Redirect {
    Redirect::to(uri!("/create.html"))
}

#[get("/contest/<id>")]
pub async fn show_contest(id: i32, mut db: Connection<DB>) -> Result<Template, Status> {
    match fetch_contest_with_ranking(&mut db, id)
        .await
        .attach_info(Status::InternalServerError, "")?
    {
        Some(contest) => Ok(Template::render("ranking", contest)),
        None => Err(Status::NotFound),
    }
}

#[get("/settings/<id>")]
async fn contest_settings(id: i32, mut db: Connection<DB>) -> Result<Template, Status> {
    match fetch_contest(&mut db, id)
        .await
        .attach_info(Status::InternalServerError, "")?
    {
        Some(contest) => Ok(Template::render("settings", contest)),
        None => Err(Status::NotFound),
    }
}

#[get("/")]
async fn show_contest_list(mut db: Connection<DB>) -> Result<Template, Status> {
    use crate::schema::contests;

    let contests = contests::dsl::contests
        .filter(contests::active.eq(true))
        .order(contests::id.desc())
        .limit(10)
        .load::<model::ContestWithId>(&mut **db)
        .await
        .map_err(|error| anyhow!("Failed to fetch contests: {}", error))
        .attach_info(Status::InternalServerError, "")?;

    Ok(Template::render("contests", context! { contests }))
}

pub fn routes() -> Vec<Route> {
    routes![create_redirect, show_contest, contest_settings, show_contest_list,]
}
