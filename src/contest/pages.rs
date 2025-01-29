use anyhow::anyhow;
use chrono::NaiveDateTime;
use diesel::{update, ExpressionMethods, QueryDsl};
use rocket::form::Form;
use rocket::http::Status;
use rocket::response::Redirect;
use rocket::{Route, State};
use rocket_db_pools::diesel::prelude::RunQueryDsl;
use rocket_db_pools::Connection;
use rocket_dyn_templates::{context, Template};

use super::fetch::fetch_contest;
use super::import::create_contest;
use crate::error::IntoStatusResult;
use crate::{model, PhiQuadroLogin, DB};

#[derive(FromForm)]
struct ContestCreationData {
    phi_id: u32,
    phi_sess: u32,
    name: String,
    start_time: String,
    duration: u16,
    drift: u32,
    drift_duration: u16,
}

#[get("/create")]
async fn create_redirect() -> Redirect {
    Redirect::to(uri!("/create.html"))
}

#[post("/create", format = "application/x-www-form-urlencoded", data = "<contest>")]
async fn create(contest: Form<ContestCreationData>, mut db: Connection<DB>, phi: &State<PhiQuadroLogin>) -> Result<Redirect, Status> {
    let start_time = NaiveDateTime::parse_from_str(&contest.start_time, "%Y-%m-%dT%H:%M")
        .map_err(|err| anyhow!("Failed to get start datetime: {}", err))
        .attach_status(Status::BadRequest)?;

    let contest_id = create_contest(
        &mut db,
        phi.inner(),
        &contest.name,
        contest.phi_id,
        contest.phi_sess,
        contest.duration as u32 * 60,
        start_time,
        contest.drift,
        contest.drift_duration as u32 * 60,
    )
    .await?;

    Ok(Redirect::to(uri!(show_contest(contest_id))))
}

#[get("/contest/<id>")]
async fn show_contest(id: i32, mut db: Connection<DB>) -> Result<Template, Status> {
    match fetch_contest(&mut db, id).await.attach_status(Status::InternalServerError)? {
        Some(contest) => Ok(Template::render("ranking", contest)),
        None => Err(Status::NotFound),
    }
}

#[delete("/contest/<id>")]
async fn delete_contest(id: i32, mut db: Connection<DB>) -> Result<Redirect, Status> {
    use crate::schema::contests;

    update(contests::dsl::contests.filter(contests::id.eq(id)))
        .set(contests::active.eq(false))
        .execute(&mut **db)
        .await
        .attach_status(Status::InternalServerError)?;

    Ok(Redirect::to(uri!(show_contest_list)))
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
        .attach_status(Status::InternalServerError)?;

    Ok(Template::render("contests", context! {
        contests
    }))
}

pub fn routes() -> Vec<Route> {
    routes![create, create_redirect, show_contest, delete_contest, show_contest_list]
}
