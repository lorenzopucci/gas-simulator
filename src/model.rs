use chrono::{Duration, NaiveDateTime};
use diesel::data_types::PgInterval;
use diesel::{Insertable, Queryable, Selectable};

#[derive(Debug, Clone, Queryable, Selectable, Insertable)]
#[diesel(table_name = crate::schema::contests)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Contest {
    pub phiquadro_id: i32,
    pub phiquadro_sess: i32,
    pub contest_name: String,
    pub duration: PgInterval,
    pub start_time: NaiveDateTime,
    pub drift: i32,
    pub drift_time: PgInterval,
}

#[derive(Debug, Clone, Copy, Queryable, Selectable, Insertable)]
#[diesel(table_name = crate::schema::questions)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Question {
    pub answer: i32,
    pub position: i32,
    pub contest_id: i32,
}

#[derive(Debug, Clone, Queryable, Selectable, Insertable)]
#[diesel(table_name = crate::schema::teams)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Team {
    pub team_name: String,
    pub is_fake: bool,
    pub position: i32,
    pub contest_id: i32,
}

#[derive(Debug, Clone, Queryable, Selectable, Insertable)]
#[diesel(table_name = crate::schema::submissions)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Submission {
    pub answer: i32,
    pub sub_time: PgInterval,
    pub team_id: i32,
    pub question_id: i32,
}

#[derive(Debug, Clone, Queryable, Selectable, Insertable)]
#[diesel(table_name = crate::schema::jollies)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Jolly {
    pub sub_time: PgInterval,
    pub team_id: i32,
    pub question_id: i32,
}

#[derive(Queryable)]
pub struct ContestSubmissions {
    pub given_answer: i32,
    pub sub_time: PgInterval,
    pub correct_answer: i32,
    pub question_pos: i32,
    pub team_pos: i32,
    pub is_fake: bool,
    pub contest_id: i32,
}

#[derive(Queryable)]
pub struct ContestJollies {
    pub sub_time: PgInterval,
    pub question_pos: i32,
    pub team_pos: i32,
    pub contest_id: i32,
}


pub fn get_duration(pg_interval: PgInterval) -> Duration {
    Duration::microseconds(pg_interval.microseconds)
        + Duration::days(pg_interval.days as i64)
        + Duration::days(pg_interval.months as i64 * 30)
}
