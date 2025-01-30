use anyhow::anyhow;
use chrono::NaiveDateTime;
use diesel::dsl::{count_star, max};
use diesel::{update, ExpressionMethods, QueryDsl};
use rocket::http::Status;
use rocket::response::Redirect;
use rocket::serde::json::Json;
use rocket::{Route, State};
use rocket_db_pools::diesel::prelude::RunQueryDsl;
use rocket_db_pools::Connection;
use serde::Deserialize;

use crate::contest::import::create_contest;
use crate::contest::pages::rocket_uri_macro_show_contest;
use crate::error::IntoStatusResult;
use crate::model::Team;
use crate::{PhiQuadroLogin, DB};

#[derive(Deserialize)]
struct ContestCreationData<'r> {
    phi_id: u32,
    phi_sess: u32,
    name: &'r str,
    start_time: &'r str,
    duration: u16,
    drift: u32,
    drift_time: u16,
}

#[derive(Deserialize)]
struct ContestUpdateData<'r> {
    start_time: &'r str,
    duration: u16,
    drift: u32,
    drift_time: u16,
}

#[derive(Deserialize)]
struct TeamCreationData<'r> {
    team_name: &'r str,
}

#[post("/create", format = "application/json", data = "<contest>")]
async fn create(
    contest: Json<ContestCreationData<'_>>,
    mut db: Connection<DB>,
    phi: &State<PhiQuadroLogin>,
) -> Result<Redirect, Status> {
    let start_time = NaiveDateTime::parse_from_str(contest.start_time, "%Y-%m-%dT%H:%M")
        .map_err(|err| anyhow!("Failed to get start datetime: {}", err))
        .attach_status(Status::BadRequest)?;

    let contest_id = create_contest(
        &mut db,
        phi.inner(),
        contest.name,
        contest.phi_id,
        contest.phi_sess,
        contest.duration as u32 * 60,
        start_time,
        contest.drift,
        contest.drift_time as u32 * 60,
    )
    .await?;

    Ok(Redirect::to(uri!(show_contest(contest_id))))
}

#[post("/contest/<id>", format = "application/json", data = "<data>")]
async fn update_contest(id: i32, data: Json<ContestUpdateData<'_>>, mut db: Connection<DB>) -> Result<Status, Status> {
    use crate::schema::contests;

    let start_time = NaiveDateTime::parse_from_str(data.start_time, "%Y-%m-%dT%H:%M")
        .map_err(|err| anyhow!("Failed to get start datetime: {}", err))
        .attach_status(Status::BadRequest)?;

    let drift: i32 = data
        .drift
        .try_into()
        .map_err(|_| anyhow!("Drift should be a reasonable value ({} given)", data.drift))
        .attach_status(Status::UnprocessableEntity)?;

    update(contests::dsl::contests.filter(contests::id.eq(id)))
        .set((
            contests::start_time.eq(start_time),
            contests::duration.eq(data.duration as i32 * 60),
            contests::drift.eq(drift),
            contests::drift_time.eq(data.drift_time as i32 * 60),
        ))
        .execute(&mut **db)
        .await
        .attach_status(Status::InternalServerError)?;

    Ok(Status::Accepted)
}

#[delete("/contest/<id>")]
async fn delete_contest(id: i32, mut db: Connection<DB>) -> Result<Status, Status> {
    use crate::schema::contests;

    update(contests::dsl::contests.filter(contests::id.eq(id)))
        .set(contests::active.eq(false))
        .execute(&mut **db)
        .await
        .attach_status(Status::InternalServerError)?;

    Ok(Status::Accepted)
}

#[post("/teams/<id>", format = "application/json", data = "<team>")]
async fn add_team(id: i32, team: Json<TeamCreationData<'_>>, mut db: Connection<DB>) -> Result<Status, Status> {
    use crate::schema::teams;

    let team_no = teams::dsl::teams
        .select(count_star())
        .filter(teams::contest_id.eq(id))
        .load::<i64>(&mut **db)
        .await
        .attach_status(Status::InternalServerError)?[0];

    diesel::insert_into(teams::dsl::teams)
        .values(Team {
            team_name: team.team_name.to_string(),
            contest_id: id,
            is_fake: false,
            position: team_no as i32,
        })
        .execute(&mut **db)
        .await
        .attach_status(Status::InternalServerError)?;

    Ok(Status::Accepted)
}

#[delete("/teams/<id>")]
async fn delete_team(id: i32, mut db: Connection<DB>) -> Result<Status, Status> {
    use crate::schema::teams;

    let pos = diesel::delete(teams::dsl::teams)
        .filter(teams::id.eq(id))
        .returning(teams::position)
        .load::<i32>(&mut **db)
        .await
        .attach_status(Status::InternalServerError)?;

    let Some(&pos) = pos.get(0) else {
        return Ok(Status::Accepted);
    };

    let max_pos = teams::dsl::teams
        .select(max(teams::position))
        .load::<Option<i32>>(&mut **db)
        .await
        .attach_status(Status::InternalServerError)?;

    let Some(Some(max_pos)) = max_pos.get(0) else {
        return Ok(Status::Accepted);
    };

    if *max_pos < pos {
        return Ok(Status::Accepted);
    }

    diesel::update(teams::dsl::teams)
        .filter(teams::position.eq(max_pos))
        .set(teams::position.eq(pos))
        .execute(&mut **db)
        .await
        .attach_status(Status::InternalServerError)?;

    Ok(Status::Accepted)
}

pub fn routes() -> Vec<Route> {
    routes![create, update_contest, delete_contest, add_team, delete_team,]
}
