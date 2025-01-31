use diesel::{ExpressionMethods, QueryDsl};
use diesel::prelude::Queryable;
use rocket::http::{HeaderMap, Status};
use rocket_db_pools::{diesel::prelude::RunQueryDsl, Connection};
use serde::{Deserialize, Serialize};

use crate::api::ApiInputResult;
use crate::DB;
use crate::error::IntoStatusResult;
use super::{ApiError, ApiResponse};


#[derive(Queryable, Serialize)]
pub struct QuestionGetResponse {
    id: i32,
    answer: i32,
}

#[derive(Deserialize)]
pub struct QuestionPostData {
    answer: i32,
}

#[get("/contests/<id>/questions")]
pub async fn get_questions<'r>(
    id: i32,
    mut db: Connection<DB>
) -> Result<ApiResponse<'r, Vec<QuestionGetResponse>>, ApiResponse<'r, ApiError>> {
    use crate::schema::{contests, questions};

    let exists: i64 = contests::dsl::contests
        .filter(contests::dsl::id.eq(id))
        .filter(contests::active.eq(true))
        .count()
        .get_result(&mut **db)
        .await
        .attach_info(Status::InternalServerError, "Errore riscontrato durante l'operazione")?;

    if exists == 0 {
        return Err(ApiResponse {
            status: Status::NotFound,
            body: ApiError { error: "".to_string(), },
            headers: HeaderMap::new(),
        });
    }

    let questions = questions::dsl::questions
        .select((questions::id, questions::answer))
        .filter(questions::contest_id.eq(id))
        .order(questions::position.asc())
        .load::<QuestionGetResponse>(&mut **db)
        .await
        .attach_info(Status::InternalServerError, "Errore riscontrato durante l'operazione")?;

    Ok(ApiResponse {
        status: Status::Ok,
        body: questions,
        headers: HeaderMap::new(),
    })
}

#[post("/contests/<id>/questions", format = "application/json", data = "<question>")]
pub async fn post_question<'r>(
    id: i32,
    question: ApiInputResult<'r, QuestionPostData>,
    mut db: Connection<DB>
) -> Status {
    Status::NotImplemented
}

#[get("/contests/<id>/questions/<question_id>")]
pub async fn get_question<'r>(
    id: i32,
    question_id: i32,
    mut db: Connection<DB>
) -> Result<ApiResponse<'r, QuestionGetResponse>, ApiResponse<'r, ApiError>> {
    use crate::schema::questions;

    let questions = questions::dsl::questions
        .select((questions::id, questions::answer))
        .filter(questions::id.eq(question_id))
        .filter(questions::contest_id.eq(id))
        .order(questions::position.asc())
        .load::<QuestionGetResponse>(&mut **db)
        .await
        .attach_info(Status::InternalServerError, "Errore riscontrato durante l'operazione")?;

    match questions.into_iter().next() {
        Some(question) => Ok(ApiResponse {
            status: Status::Ok,
            body: question,
            headers: HeaderMap::new(),
        }),
        None => Err(ApiResponse {
            status: Status::NotFound,
            body: ApiError { error: "".to_string() },
            headers: HeaderMap::new(),
        }),
    }
}

#[delete("/contests/<id>/questions/<question_id>")]
pub async fn delete_question<'r>(
    id: i32,
    question_id: i32,
    mut db: Connection<DB>
) -> Status {
    Status::NotImplemented
}

