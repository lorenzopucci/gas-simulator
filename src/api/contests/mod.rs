use anyhow::anyhow;
use chrono::NaiveDateTime;
use diesel::{update, ExpressionMethods, QueryDsl};
use diesel::prelude::{AsChangeset, Queryable};
use reqwest::header;
use rocket::http::{Header, HeaderMap, Status};
use rocket::State;
use rocket_db_pools::{diesel::prelude::RunQueryDsl, Connection};
use serde::{Deserialize, Serialize};

use crate::{PhiQuadroLogin, DB};
use crate::contest::import::create_contest;
use crate::error::IntoStatusResult;
use super::{ApiError, ApiInputResult, ApiResponse, ApiUser};

pub mod jollies;
pub mod teams;
pub mod questions;
pub mod submissions;

#[derive(Deserialize)]
pub struct ContestPostData<'r> {
    phiquadro_id: u32,
    phiquadro_sess: u32,
    name: &'r str,
    start_time: &'r str,
    duration: u16,
    drift: u32,
    drift_time: u16,
}

#[derive(Serialize)]
pub struct ContestPostResponse {
    contest_id: i32,
}

#[derive(Queryable, Serialize)]
pub struct ContestGetResponse {
    phiquadro_id: i32,
    phiquadro_sess: i32,
    name: String,
    duration: i32,
    start_time: NaiveDateTime,
    drift: i32,
    drift_time: i32,
}

#[derive(Deserialize)]
pub struct ContestPatchData<'r> {
    start_time: Option<&'r str>,
    duration: Option<u16>,
    drift: Option<u32>,
    drift_time: Option<u16>,
}

#[derive(AsChangeset)]
#[diesel(table_name = crate::schema::contests)]
pub struct ContestUpdateForm {
    pub start_time: Option<NaiveDateTime>,
    pub duration: Option<i32>,
    pub drift: Option<i32>,
    pub drift_time: Option<i32>,
}

#[post("/contests", format = "application/json", data = "<contest>")]
pub async fn post_contest<'r>(
    contest: ApiInputResult<'r, ContestPostData<'r>>,
    mut db: Connection<DB>,
    phi: &State<PhiQuadroLogin>,
    user: ApiUser,
) -> Result<ApiResponse<'r, ContestPostResponse>, ApiResponse<'r, ApiError>> {
    let Ok(contest) = contest else {
        return Err(ApiResponse {
            status: Status::BadRequest,
            body: ApiError { error: "Richiesta malformata".to_string() },
            headers: HeaderMap::new(),
        });
    };

    let start_time = NaiveDateTime::parse_from_str(contest.start_time, "%Y-%m-%dT%H:%M")
        .map_err(|err| anyhow!("Failed to get start datetime: {}", err))
        .attach_info(Status::BadRequest, "Ora di inizio non valida")?;

    let contest_id = create_contest(
        &mut db,
        phi.inner(),
        user.user_id,
        contest.name,
        contest.phiquadro_id,
        contest.phiquadro_sess,
        contest.duration as u32 * 60,
        start_time,
        contest.drift,
        contest.drift_time as u32 * 60,
    )
    .await?;

    let mut headers = HeaderMap::new();
    headers.add(Header::new(header::LOCATION.as_str(), format!("/contest/{contest_id}")));

    Ok(ApiResponse {
        status: Status::Created,
        body: ContestPostResponse { contest_id },
        headers,
    })
}

#[get("/contests/<id>")]
pub async fn get_contest<'r>(
    id: i32,
    mut db: Connection<DB>
) -> Result<ApiResponse<'r, ContestGetResponse>, ApiResponse<'r, ApiError>> {
    use crate::schema::contests;

    let contest = contests::dsl::contests
        .select((
            contests::phiquadro_id,
            contests::phiquadro_sess,
            contests::contest_name,
            contests::duration,
            contests::start_time,
            contests::drift,
            contests::drift_time,
        ))
        .filter(contests::dsl::id.eq(id))
        .filter(contests::active.eq(true))
        .load::<ContestGetResponse>(&mut **db)
        .await
        .attach_info(Status::InternalServerError, "Errore riscontrato durante l'operazione")?;

    match contest.into_iter().next() {
        Some(contest) => Ok(ApiResponse {
            status: Status::Ok,
            body: contest,
            headers: HeaderMap::new(),
        }),
        None => Err(ApiResponse {
            status: Status::NotFound,
            body: ApiError { error: "".to_string() },
            headers: HeaderMap::new(),
        }),
    }
}

#[patch("/contests/<id>", format = "application/json", data = "<data>")]
pub async fn patch_contest<'r>(
    id: i32,
    data: ApiInputResult<'r, ContestPatchData<'r>>,
    mut db: Connection<DB>
) -> Result<ApiResponse<'r, ()>, ApiResponse<'r, ApiError>> {
    use crate::schema::contests;

    let Ok(data) = data else {
        return Err(ApiResponse {
            status: Status::BadRequest,
            body: ApiError { error: "Richiesta malformata".to_string() },
            headers: HeaderMap::new(),
        });
    };

    let start_time = data.start_time.map(|start_time|
        NaiveDateTime::parse_from_str(start_time, "%Y-%m-%dT%H:%M")
            .map_err(|err| anyhow!("Failed to get start datetime: {}", err))
            .attach_info(Status::BadRequest, "Ora di inizio non valida")
    )
    .transpose()?;

    let drift = data.drift.map(|drift| drift
        .try_into()
        .map_err(|_| anyhow!("Drift should be a reasonable value ({} given)", drift))
        .attach_info(Status::UnprocessableEntity, "Deriva non valida")
    )
    .transpose()?;

    let duration = data.duration.map(|duration| duration as i32 * 60);
    let drift_time = data.drift_time.map(|drift_time| drift_time as i32 * 60);

    update(contests::dsl::contests.filter(contests::id.eq(id)))
        .set(&ContestUpdateForm {
            start_time,
            duration,
            drift,
            drift_time,
        })
        .execute(&mut **db)
        .await
        .attach_info(Status::InternalServerError, "Errore incontrato durante l'aggiornamento delle impostazioni")?;

    Ok(ApiResponse {
        status: Status::NoContent,
        body: (),
        headers: HeaderMap::new(),
    })
}

#[delete("/contests/<id>")]
pub async fn delete_contest<'r>(id: i32, mut db: Connection<DB>) -> Result<ApiResponse<'r, ()>, ApiResponse<'r, ApiError>> {
    use crate::schema::contests;

    update(contests::dsl::contests.filter(contests::id.eq(id)))
        .set(contests::active.eq(false))
        .execute(&mut **db)
        .await
        .attach_info(Status::InternalServerError, "Errore incontrato durante l'eliminazione della gara")?;

    Ok(ApiResponse {
        status: Status::NoContent,
        body: (),
        headers: HeaderMap::new(),
    })
}

