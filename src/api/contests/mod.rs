use anyhow::anyhow;
use chrono::{DateTime, NaiveDateTime, TimeZone, Utc};
use chrono_tz::Europe::Rome;
use diesel::sql_types::{Integer, Interval};
use diesel::{sql_query, update, ExpressionMethods, QueryDsl};
use diesel::prelude::{AsChangeset, Queryable};
use reqwest::header;
use rocket::http::{Header, HeaderMap, Status};
use rocket::State;
use rocket_db_pools::{diesel::prelude::RunQueryDsl, Connection};
use serde::{Deserialize, Serialize};

use crate::model::timedelta_to_pg_interval;
use crate::{PhiQuadroLogin, DB};
use crate::contest::import::create_contest;
use crate::error::IntoStatusResult;
use super::{ApiError, ApiInputResult, ApiResponse, ApiUser};

pub mod jollies;
pub mod teams;
pub mod submissions;

#[derive(Serialize)]
pub struct ContestsGetResponse {
    contests: Vec<i32>,
}

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
    pub start_time: Option<DateTime<Utc>>,
    pub duration: Option<i32>,
    pub drift: Option<i32>,
    pub drift_time: Option<i32>,
}

#[get("/contests")]
pub async fn get_contests<'r>(
    mut db: Connection<DB>,
    api_user: ApiUser,
) -> Result<ApiResponse<'r, ContestsGetResponse>, ApiResponse<'r, ApiError>> {
    use crate::schema::contests;

    let contest_list = contests::dsl::contests
        .select(contests::id)
        .filter(contests::owner_id.eq(api_user.user_id))
        .filter(contests::active.eq(true))
        .load(&mut **db)
        .await
        .attach_info(Status::InternalServerError, "Errore riscontrato durante l'operazione")?;

    Ok(ApiResponse {
        status: Status::Unauthorized,
        body: ContestsGetResponse { contests: contest_list },
        headers: HeaderMap::new(),
    })
}

#[post("/contests", format = "application/json", data = "<contest>")]
pub async fn post_contest<'r>(
    contest: ApiInputResult<'r, ContestPostData<'r>>,
    mut db: Connection<DB>,
    phi: &State<PhiQuadroLogin>,
    api_user: ApiUser,
) -> Result<ApiResponse<'r, ContestPostResponse>, ApiResponse<'r, ApiError>> {
    let Ok(contest) = contest else {
        return Err(ApiResponse {
            status: Status::BadRequest,
            body: ApiError { error: "Richiesta malformata".to_string() },
            headers: HeaderMap::new(),
        });
    };

    let start_time = Rome.from_local_datetime(
        &NaiveDateTime::parse_from_str(contest.start_time, "%Y-%m-%dT%H:%M")
            .map_err(|err| anyhow!("Failed to get start datetime: {}", err))
            .attach_info(Status::BadRequest, "Ora di inizio non valida")?
    )
    .unwrap()
    .with_timezone(&Utc);

    if start_time <= chrono::offset::Utc::now() {
        return Err(ApiResponse {
            status: Status::UnprocessableEntity,
            body: ApiError { error: "La gara non può iniziare nel passato".to_string() },
            headers: HeaderMap::new(),
        });
    }

    let contest_id = create_contest(
        &mut db,
        phi.inner(),
        api_user.user_id,
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
    mut db: Connection<DB>,
    api_user: ApiUser,
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
        .filter(contests::owner_id.eq(api_user.user_id))
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
            body: ApiError { error: "La gara non esiste o non ti appartiene".to_string() },
            headers: HeaderMap::new(),
        }),
    }
}

#[patch("/contests/<id>", format = "application/json", data = "<data>")]
pub async fn patch_contest<'r>(
    id: i32,
    data: ApiInputResult<'r, ContestPatchData<'r>>,
    mut db: Connection<DB>,
    api_user: ApiUser,
) -> Result<ApiResponse<'r, ()>, ApiResponse<'r, ApiError>> {
    use crate::schema::contests;

    let Ok(data) = data else {
        return Err(ApiResponse {
            status: Status::BadRequest,
            body: ApiError { error: "Richiesta malformata".to_string() },
            headers: HeaderMap::new(),
        });
    };

    let start_time = data.start_time.map(|start_time| {
        let input_datetime = Rome.from_local_datetime(
            &NaiveDateTime::parse_from_str(start_time, "%Y-%m-%dT%H:%M")
                .map_err(|err| anyhow!("Failed to get start datetime: {}", err))
                .attach_info(Status::BadRequest, "Ora di inizio non valida")?
        )
        .unwrap()
        .with_timezone(&Utc);

        let out = if input_datetime <= chrono::offset::Utc::now() {
            Err(anyhow!("Contest can't start in the past"))
        } else {
            Ok(input_datetime)
        };

        out.attach_info(Status::UnprocessableEntity, "La gara non può iniziare nel passato")
    })
    .transpose()?;

    let drift = data.drift.map(|drift| drift
        .try_into()
        .map_err(|_| anyhow!("Drift should be a reasonable value ({} given)", drift))
        .attach_info(Status::UnprocessableEntity, "Deriva non valida")
    )
    .transpose()?;

    let duration = data.duration.map(|duration| duration as i32 * 60);
    let drift_time = data.drift_time.map(|drift_time| drift_time as i32 * 60);

    let contest_start_time = contests::dsl::contests
        .select(contests::start_time)
        .filter(contests::id.eq(id))
        .filter(contests::active.eq(true))
        .filter(contests::owner_id.eq(api_user.user_id))
        .load::<DateTime<Utc>>(&mut **db)
        .await
        .attach_info(Status::InternalServerError, "Errore incontrato durante l'aggiornamento delle impostazioni")?;

    let Some(&contest_start_time) = contest_start_time.get(0) else {
        return Err(ApiResponse {
            status: Status::NotFound,
            body: ApiError { error: "La gara non esiste o non ti appartiene".to_string() },
            headers: HeaderMap::new(),
        });
    };

    if contest_start_time <= chrono::offset::Utc::now() {
        return Err(ApiResponse {
            status: Status::Forbidden,
            body: ApiError { error: "La gara è già iniziata".to_string() },
            headers: HeaderMap::new(),
        });
    }

    update(
        contests::dsl::contests
            .filter(contests::id.eq(id))
            .filter(contests::owner_id.eq(api_user.user_id))
    )
        .set(&ContestUpdateForm {
            start_time,
            duration,
            drift,
            drift_time,
        })
        .execute(&mut **db)
        .await
        .attach_info(Status::InternalServerError, "Errore incontrato durante l'aggiornamento delle impostazioni")?;

    if let Some(start_time) = start_time {
        let delta = timedelta_to_pg_interval(start_time - contest_start_time);
        sql_query(include_str!("update_times_submissions.sql"))
            .bind::<Interval, _>(delta)
            .bind::<Integer, _>(id)
            .execute(&mut **db)
            .await
            .attach_info(Status::InternalServerError, "Errore incontrato durante l'aggiornamento delle impostazioni")?;
        sql_query(include_str!("update_times_jollies.sql"))
            .bind::<Interval, _>(delta)
            .bind::<Integer, _>(id)
            .execute(&mut **db)
            .await
            .attach_info(Status::InternalServerError, "Errore incontrato durante l'aggiornamento delle impostazioni")?;
    }

    Ok(ApiResponse {
        status: Status::NoContent,
        body: (),
        headers: HeaderMap::new(),
    })
}

#[delete("/contests/<id>")]
pub async fn delete_contest<'r>(
    id: i32,
    mut db: Connection<DB>,
    api_user: ApiUser,
) -> Result<ApiResponse<'r, ()>, ApiResponse<'r, ApiError>> {
    use crate::schema::contests;

    let changes = update(
        contests::dsl::contests
            .filter(contests::id.eq(id))
            .filter(contests::active.eq(true))
            .filter(contests::owner_id.eq(api_user.user_id))
    )
        .set(contests::active.eq(false))
        .execute(&mut **db)
        .await
        .attach_info(Status::InternalServerError, "Errore incontrato durante l'eliminazione della gara")?;

    if changes == 0 {
        return Err(ApiResponse {
            status: Status::NotFound,
            body: ApiError { error: "La gara non esiste o non ti appartiene".to_string() },
            headers: HeaderMap::new(),
        })
    }

    Ok(ApiResponse {
        status: Status::NoContent,
        body: (),
        headers: HeaderMap::new(),
    })
}

