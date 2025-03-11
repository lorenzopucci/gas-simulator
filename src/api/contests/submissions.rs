use chrono::{DateTime, Duration, Utc};
use diesel::prelude::Queryable;
use diesel::{ExpressionMethods, QueryDsl};
use rocket::http::hyper::header;
use rocket::http::{Header, HeaderMap, Status};
use rocket::serde::json::Json;
use rocket_db_pools::diesel::prelude::RunQueryDsl;
use rocket_db_pools::Connection;
use serde::{Deserialize, Serialize};

use crate::api::{ApiError, ApiInputResult, ApiResponse, ApiUser};
use crate::error::IntoStatusResult;
use crate::model::Submission;
use crate::DB;

#[derive(Serialize)]
pub struct SubmissionsGetResponse {
    submissions: Vec<i32>,
}

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

#[derive(Serialize)]
pub struct SubmissionPostResponse {
    submission_id: i32,
    correct: bool,
}

#[derive(Deserialize)]
pub struct SubmissionPatchData {
    answer: i32,
    team_id: i32,
    question_id: i32,
}

#[get("/contests/<id>/submissions")]
pub async fn get_submissions<'r>(
    id: i32,
    mut db: Connection<DB>,
    api_user: ApiUser,
) -> Result<ApiResponse<'r, SubmissionsGetResponse>, ApiResponse<'r, ApiError>> {
    use crate::schema::{contests, teams, submissions};

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

    let submissions = submissions::dsl::submissions
        .inner_join(teams::table)
        .select(submissions::id)
        .filter(teams::contest_id.eq(id))
        .load::<i32>(&mut **db)
        .await
        .attach_info(Status::InternalServerError, "Errore riscontrato durante l'operazione")?;

    Ok(ApiResponse {
        status: Status::Ok,
        body: SubmissionsGetResponse { submissions },
        headers: HeaderMap::new(),
    })
}

#[post("/contests/<id>/submissions", format = "application/json", data = "<submission>")]
pub async fn post_submission<'r>(
    id: i32,
    submission: ApiInputResult<'_, SubmissionPostData>,
    mut db: Connection<DB>,
    api_user: ApiUser,
) -> Result<ApiResponse<'r, SubmissionPostResponse>, ApiResponse<'r, ApiError>> {
    use crate::schema::{contests, teams, questions, submissions};

    let Ok(submission) = submission else {
        return Err(ApiResponse {
            status: Status::BadRequest,
            body: ApiError { error: "Richiesta malformata".to_string() },
            headers: HeaderMap::new(),
        });
    };

    let contest = contests::dsl::contests
        .select((contests::owner_id, contests::start_time, contests::duration))
        .filter(contests::id.eq(id))
        .filter(contests::active.eq(true))
        .load::<(i32, DateTime<Utc>, i32)>(&mut **db)
        .await
        .attach_info(Status::InternalServerError, "Errore incontrato durante l'invio della risposta")?;

    let Some(&(contest_owner, start, duration)) = contest.get(0) else {
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
        .filter(teams::id.eq(submission.team_id))
        .load::<i64>(&mut **db)
        .await
        .attach_info(Status::InternalServerError, "Errore incontrato durante l'invio della risposta")?;

    let question = questions::dsl::questions
        .select(questions::answer)
        .filter(questions::id.eq(submission.question_id))
        .load::<i32>(&mut **db)
        .await
        .attach_info(Status::InternalServerError, "Errore incontrato durante l'invio della risposta")?;

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

    if curr_time > start + Duration::seconds(duration as i64) {
        return Err(ApiResponse {
            status: Status::Forbidden,
            body: ApiError { error: "Il tempo per la consegna delle risposte è scaduto".to_string() },
            headers: HeaderMap::new(),
        });
    }

    let submission_id = diesel::insert_into(submissions::dsl::submissions)
        .values(Submission {
            answer: submission.answer,
            sub_time: chrono::Utc::now(),
            team_id: submission.team_id,
            question_id: submission.question_id,
        })
        .returning(submissions::id)
        .get_result(&mut **db)
        .await
        .attach_info(Status::InternalServerError, "Errore incontrato durante l'invio della risposta")?;

    let mut headers = HeaderMap::new();
    headers.add(Header::new(header::LOCATION.as_str(), format!("/contest/{id}/submissions/{submission_id}")));

    Ok(ApiResponse {
        status: Status::Created,
        body: SubmissionPostResponse { submission_id, correct: answer == submission.answer },
        headers,
    })
}

#[get("/contests/<id>/submissions/<submission_id>")]
pub async fn get_submission<'r>(
    id: i32,
    submission_id: i32,
    mut db: Connection<DB>,
    api_user: ApiUser,
) -> Result<ApiResponse<'r, SubmissionGetResponse>, ApiResponse<'r, ApiError>> {
    use crate::schema::{contests, teams, submissions};

    let submissions = submissions::dsl::submissions
        .inner_join(teams::table.inner_join(contests::table))
        .select((submissions::id, submissions::answer, submissions::team_id, submissions::question_id))
        .filter(submissions::id.eq(submission_id))
        .filter(teams::contest_id.eq(id))
        .filter(contests::owner_id.eq(api_user.user_id))
        .load::<SubmissionGetResponse>(&mut **db)
        .await
        .attach_info(Status::InternalServerError, "Errore riscontrato durante l'operazione")?;

    match submissions.into_iter().next() {
        Some(submission) => Ok(ApiResponse {
            status: Status::Ok,
            body: submission,
            headers: HeaderMap::new(),
        }),
        None => Err(ApiResponse {
            status: Status::NotFound,
            body: ApiError { error: "La sottoposizione non esiste o non ti appartiene".to_string() },
            headers: HeaderMap::new(),
        }),
    }
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

