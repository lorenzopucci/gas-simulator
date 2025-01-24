use anyhow::Result;
use chrono::NaiveDateTime;
use reqwest::Client;

use rocket_db_pools::diesel::prelude::RunQueryDsl;
use rocket_db_pools::Connection;

use ::diesel::data_types::PgInterval;
use ::diesel::dsl::count_star;
use ::diesel::query_dsl::methods::SelectDsl;

use crate::models::{Contest, Question, Team};
use crate::schema;
use crate::DB;

const COOKIE: &str = "PHPSESSID=";
const STATS_URL: &str = "https://www.phiquadro.it/gara_a_squadre/stampe/statistiche_squadra.php";

pub async fn create_contest(
    mut db: Connection<DB>,
    name: &str,
    id: i32,
    sess: i32,
    duration: u32,
    start_time: NaiveDateTime,
    drift: i32,
) -> Result<i32> {
    use self::schema::contests;
    let contest_id = contests::dsl::contests
        .select(count_star())
        .get_result::<i64>(&mut **db)
        .await? as i32;

    diesel::insert_into(contests::table)
        .values(&Contest {
            id: contest_id,
            contest_name: name.to_string(),
            phiquadro_id: id,
            phiquadro_sess: sess,
            duration: PgInterval::from_microseconds((duration as i64) * 1_000_000),
            start_time,
            drift,
        })
        .execute(&mut **db)
        .await?;

    Ok(contest_id)
}

pub async fn import_contest(id_gara: u32, id_sess: u32) -> Result<()> {
    let client = Client::new();

    let contest_pdf = client
        .post(STATS_URL)
        .header("Cookie", COOKIE)
        .form(&[("id_gara", id_gara), ("id_sess", id_sess)])
        .send()
        .await?
        .bytes()
        .await?;

    std::fs::write("sus.pdf", contest_pdf)?;

    Ok(())
}
