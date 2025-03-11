use chrono::{DateTime, Duration, Utc};
use diesel::prelude::Queryable;
use diesel::{ExpressionMethods, QueryDsl};
use reqwest::header;
use rocket::http::{Header, HeaderMap, Status};
use rocket::serde::json::Json;
use rocket_db_pools::diesel::prelude::RunQueryDsl;
use rocket_db_pools::Connection;
use serde::{Deserialize, Serialize};

use crate::api::{ApiError, ApiInputResult, ApiResponse, ApiUser};
use crate::error::IntoStatusResult;
use crate::model::Jolly;
use crate::DB;

#[derive(Serialize)]
pub struct JolliesGetResponse {
    jollies: Vec<i32>,
}

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

#[derive(Serialize)]
pub struct JollyPostResponse {
    jolly_id: i32,
}

#[derive(Deserialize)]
pub struct JollyPatchData {
    team_id: i32,
    question_id: i32,
}

#[get("/contests/<id>/jollies")]
pub async fn get_jollies<'r>(
    id: i32,
    mut db: Connection<DB>,
    api_user: ApiUser,
) -> Result<ApiResponse<'r, JolliesGetResponse>, ApiResponse<'r, ApiError>> {
    use crate::schema::{contests, teams, jollies};

    let exists: i64 = contests::dsl::contests
        .filter(contests::id.eq(id))
        .filter(contests::owner_id.eq(api_user.user_id))
        .filter(contests::active.eq(true))
        .count()
        .get_result(&mut **db)
        .await
        .attach_info(Status::InternalServerError, "Errore riscontrato durante l'operazione")?;

    if exists == 0 {
        return Err(ApiResponse {
            status: Status::NotFound,
            body: ApiError { error: "La gara non esiste o non ti appartiene".to_string(), },
            headers: HeaderMap::new(),
        });
    }

    let jollies = jollies::dsl::jollies
        .inner_join(teams::table)
        .select(jollies::id)
        .filter(teams::contest_id.eq(id))
        .load::<i32>(&mut **db)
        .await
        .attach_info(Status::InternalServerError, "Errore riscontrato durante l'operazione")?;

    Ok(ApiResponse {
        status: Status::Ok,
        body: JolliesGetResponse { jollies },
        headers: HeaderMap::new(),
    })
}

#[post("/contests/<id>/jollies", format = "application/json", data = "<jolly>")]
pub async fn post_jolly<'r>(
    id: i32,
    jolly: ApiInputResult<'_, JollyPostData>,
    mut db: Connection<DB>,
    api_user: ApiUser,
) -> Result<ApiResponse<'r, JollyPostResponse>, ApiResponse<'r, ApiError>> {
    use crate::schema::{contests, teams, questions, jollies};

    let Ok(jolly) = jolly else {
        return Err(ApiResponse {
            status: Status::BadRequest,
            body: ApiError { error: "Richiesta malformata".to_string() },
            headers: HeaderMap::new(),
        });
    };

    let contest = contests::dsl::contests
        .select((contests::owner_id, contests::start_time, contests::jolly_time))
        .filter(contests::id.eq(id))
        .filter(contests::active.eq(true))
        .load::<(i32, DateTime<Utc>, i32)>(&mut **db)
        .await
        .attach_info(Status::InternalServerError, "Errore incontrato durante l'invio del jolly")?;

    let Some(&(contest_owner, start, jolly_time)) = contest.get(0) else {
        return Err(ApiResponse {
            status: Status::NotFound,
            body: ApiError { error: "La gara non esiste o non ti appartiene".to_string() },
            headers: HeaderMap::new(),
        });
    };

    if contest_owner != api_user.user_id {
        return Err(ApiResponse {
            status: Status::NotFound,
            body: ApiError { error: "La gara non esiste o non ti appartiene".to_string() },
            headers: HeaderMap::new(),
        });
    }

    let team = teams::dsl::teams
        .count()
        .filter(teams::id.eq(jolly.team_id))
        .load::<i64>(&mut **db)
        .await
        .attach_info(Status::InternalServerError, "Errore incontrato durante l'invio del jolly")?;

    let question = questions::dsl::questions
        .select(questions::answer)
        .filter(questions::id.eq(jolly.question_id))
        .load::<i32>(&mut **db)
        .await
        .attach_info(Status::InternalServerError, "Errore incontrato durante l'invio del jolly")?;

    if team[0] == 0 {
        return Err(ApiResponse {
            status: Status::NotFound,
            body: ApiError { error: "La squadra non esiste o non ti appartiene".to_string() },
            headers: HeaderMap::new(),
        });
    }

    let Some(&answer) = question.get(0) else {
        return Err(ApiResponse {
            status: Status::NotFound,
            body: ApiError { error: "La domanda non esiste o non ti appartiene".to_string() },
            headers: HeaderMap::new(),
        });
    };

    let curr_time = chrono::Utc::now();
    if curr_time < start {
        return Err(ApiResponse {
            status: Status::Forbidden,
            body: ApiError { error: "La gara non è ancora iniziata".to_string() },
            headers: HeaderMap::new(),
        });
    }

    if curr_time > start + Duration::seconds(jolly_time as i64) {
        return Err(ApiResponse {
            status: Status::Forbidden,
            body: ApiError { error: "Il tempo per la consegna del jolly è scaduto".to_string() },
            headers: HeaderMap::new(),
        });
    }

    let jolly_id = diesel::insert_into(jollies::dsl::jollies)
        .values(Jolly {
            sub_time: chrono::Utc::now(),
            team_id: jolly.team_id,
            question_id: jolly.question_id,
        })
        .returning(jollies::id)
        .get_result(&mut **db)
        .await
        .attach_info(Status::Forbidden, "Non puoi scegliere due volte il jolly!")?;

    let mut headers = HeaderMap::new();
    headers.add(Header::new(header::LOCATION.as_str(), format!("/contest/{id}/jollies/{jolly_id}")));

    Ok(ApiResponse {
        status: Status::Created,
        body: JollyPostResponse { jolly_id },
        headers,
    })
}

#[get("/contests/<id>/jollies/<jolly_id>")]
pub async fn get_jolly<'r>(
    id: i32,
    jolly_id: i32,
    mut db: Connection<DB>,
    api_user: ApiUser,
) -> Result<ApiResponse<'r, JollyGetResponse>, ApiResponse<'r, ApiError>> {
    use crate::schema::{contests, teams, jollies};

    let jollies = jollies::dsl::jollies
        .inner_join(teams::table.inner_join(contests::table))
        .select((jollies::id, jollies::team_id, jollies::question_id))
        .filter(jollies::id.eq(jolly_id))
        .filter(teams::contest_id.eq(id))
        .filter(contests::owner_id.eq(api_user.user_id))
        .load::<JollyGetResponse>(&mut **db)
        .await
        .attach_info(Status::InternalServerError, "Errore riscontrato durante l'operazione")?;

    match jollies.into_iter().next() {
        Some(submission) => Ok(ApiResponse {
            status: Status::Ok,
            body: submission,
            headers: HeaderMap::new(),
        }),
        None => Err(ApiResponse {
            status: Status::NotFound,
            body: ApiError { error: "Il jolly non esiste o non ti appartiene".to_string() },
            headers: HeaderMap::new(),
        }),
    }
}

#[patch("/contests/<id>/jollies/<jolly_id>", format = "application/json", data = "<data>")]
pub async fn patch_submission(
    id: i32,
    jolly_id: i32,
    data: Json<JollyPatchData>,
    mut db: Connection<DB>
) -> Status {
    Status::NotImplemented
}

#[delete("/contests/<id>/jollies/<jolly_id>")]
pub async fn delete_jolly(
    id: i32,
    jolly_id: i32,
    mut db: Connection<DB>
) -> Status {
    Status::NotImplemented
}

