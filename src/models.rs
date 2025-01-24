use chrono::NaiveDateTime;
use diesel::data_types::PgInterval;
use diesel::{Insertable, Queryable, Selectable};

#[derive(Clone, Queryable, Selectable, Insertable)]
#[diesel(table_name = crate::schema::contests)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Contest {
    pub id: i32,
    pub contest_name: String,
    pub phiquadro_id: i32,
    pub phiquadro_sess: i32,
    pub duration: PgInterval,
    pub start_time: NaiveDateTime,
    pub drift: i32,
}

#[derive(Clone, Copy, Queryable, Selectable, Insertable)]
#[diesel(table_name = crate::schema::questions)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Question {
    pub id: i32,
    pub answer: i32,
    pub position: i32,
    pub contest_id: i32,
}

#[derive(Clone, Queryable, Selectable, Insertable)]
#[diesel(table_name = crate::schema::teams)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Team {
    pub id: i32,
    pub team_name: String,
    pub is_fake: bool,
    pub contest_id: i32,
}

#[derive(Clone, Queryable, Selectable, Insertable)]
#[diesel(table_name = crate::schema::submissions)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Submission {
    pub id: i32,
    pub answer: i32,
    pub sub_time: PgInterval,
    pub question_id: i32,
}

#[derive(Clone, Queryable, Selectable, Insertable)]
#[diesel(table_name = crate::schema::jollies)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Jolly {
    pub id: i32,
    pub sub_time: PgInterval,
    pub question_id: i32,
}
