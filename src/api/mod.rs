use anyhow::anyhow;
use chrono::NaiveDateTime;
use diesel::dsl::{count_star, max};
use diesel::{update, ExpressionMethods, QueryDsl};
use rocket::data::FromData;
use rocket::http::hyper::header;
use rocket::http::{ContentType, Header, HeaderMap, Status};
use rocket::response::{self, Responder};
use rocket::serde::json::Json;
use rocket::{Request, Response, Route, State};
use rocket_db_pools::diesel::prelude::RunQueryDsl;
use rocket_db_pools::Connection;
use serde::{Deserialize, Serialize};

use crate::contest::import::create_contest;
use crate::error::IntoStatusResult;
use crate::model::{ContestUpdateForm, Team};
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

#[derive(Serialize)]
struct ContestCreationResponse {
    contest_id: i32,
}

#[derive(Serialize)]
struct TeamCreationResponse {
    team_id: i32,
}


#[derive(Deserialize)]
struct ContestUpdateData<'r> {
    start_time: Option<&'r str>,
    duration: Option<u16>,
    drift: Option<u32>,
    drift_time: Option<u16>,
}

#[derive(Deserialize)]
struct TeamCreationData<'r> {
    team_name: &'r str,
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

type ApiInputResult<'r, T> = Result<Json<T>, <Json<T> as FromData<'r>>::Error>;

#[post("/contests", format = "application/json", data = "<contest>")]
async fn create<'r>(
    contest: ApiInputResult<'r, ContestCreationData<'r>>,
    mut db: Connection<DB>,
    phi: &State<PhiQuadroLogin>,
) -> Result<ApiResponse<'r, ContestCreationResponse>, ApiResponse<'r, ApiError>> {
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
        contest.name,
        contest.phi_id,
        contest.phi_sess,
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
        body: ContestCreationResponse { contest_id },
        headers,
    })
}

#[patch("/contests/<id>", format = "application/json", data = "<data>")]
async fn update_contest<'r>(
    id: i32,
    data: ApiInputResult<'r, ContestUpdateData<'r>>,
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
async fn delete_contest<'r>(id: i32, mut db: Connection<DB>) -> Result<ApiResponse<'r, ()>, ApiResponse<'r, ApiError>> {
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

#[post("/contests/<id>/teams", format = "application/json", data = "<team>")]
async fn add_team<'r>(
    id: i32,
    team: ApiInputResult<'r, TeamCreationData<'r>>,
    mut db: Connection<DB>
) -> Result<ApiResponse<'r, TeamCreationResponse>, ApiResponse<'r, ApiError>> {
    use crate::schema::teams;

    let Ok(team) = team else {
        return Err(ApiResponse {
            status: Status::BadRequest,
            body: ApiError { error: "Richiesta malformata".to_string() },
            headers: HeaderMap::new(),
        });
    };

    let team_no = teams::dsl::teams
        .select(count_star())
        .filter(teams::contest_id.eq(id))
        .load::<i64>(&mut **db)
        .await
        .attach_info(Status::InternalServerError, "Errore incontrato durante la creazione della squadra")?[0];

    let team_id = diesel::insert_into(teams::dsl::teams)
        .values(Team {
            team_name: team.team_name.to_string(),
            contest_id: id,
            is_fake: false,
            position: team_no as i32,
        })
        .returning(teams::id)
        .get_result(&mut **db)
        .await
        .attach_info(Status::InternalServerError, "Errore incontrato durante la creazione della squadra")?;

    let mut headers = HeaderMap::new();
    headers.add(Header::new(header::LOCATION.as_str(), format!("/contest/{id}/teams/{team_id}")));

    Ok(ApiResponse {
        status: Status::Created,
        body: TeamCreationResponse { team_id },
        headers,
    })
}

#[delete("/contests/<contest_id>/teams/<id>")]
async fn delete_team<'r>(
    contest_id: i32,
    id: i32,
    mut db: Connection<DB>
) -> Result<ApiResponse<'r, ()>, ApiResponse<'r, ApiError>> {
    use crate::schema::teams;

    let pos = diesel::delete(teams::dsl::teams)
        .filter(teams::id.eq(id))
        .filter(teams::contest_id.eq(contest_id))
        .returning(teams::position)
        .load::<i32>(&mut **db)
        .await
        .attach_info(Status::InternalServerError, "Errore incontrato durante l'eliminazione della squadra")?;

    let Some(&pos) = pos.get(0) else {
        return Ok(ApiResponse {
            status: Status::NotFound,
            body: (),
            headers: HeaderMap::new(),
        });
    };

    let max_pos = teams::dsl::teams
        .select(max(teams::position))
        .load::<Option<i32>>(&mut **db)
        .await
        .attach_info(Status::InternalServerError, "Errore incontrato durante l'eliminazione della squadra")?;

    let resp = ApiResponse {
        status: Status::NoContent,
        body: (),
        headers: HeaderMap::new(),
    };

    let Some(Some(max_pos)) = max_pos.get(0) else {
        return Ok(resp);
    };

    if *max_pos < pos {
        return Ok(resp);
    }

    diesel::update(teams::dsl::teams)
        .filter(teams::position.eq(max_pos))
        .set(teams::position.eq(pos))
        .execute(&mut **db)
        .await
        .attach_info(Status::InternalServerError, "Errore incontrato durante l'eliminazione della squadra")?;

    Ok(resp)
}

pub fn routes() -> Vec<Route> {
    routes![create, update_contest, delete_contest, add_team, delete_team,]
}
