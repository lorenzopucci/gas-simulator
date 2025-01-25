use std::env;
use std::sync::Arc;

use anyhow::{anyhow, Context};
use chrono::NaiveDateTime;
use lazy_static::lazy_static;
use reqwest::cookie::Jar;
use reqwest::Client;

use rocket::http::Status;
use rocket_db_pools::diesel::prelude::RunQueryDsl;
use rocket_db_pools::Connection;

use ::diesel::data_types::PgInterval;
use ::diesel::dsl::count_star;
use ::diesel::query_dsl::methods::SelectDsl;
use scraper::{Html, Selector};

use crate::models::{Contest, Question, Team};
use crate::schema;
use crate::{DB, PhiQuadroLogin};

use crate::error::IntoStatusResult;

const LOGIN_URL: &str = "https://www.phiquadro.it/gara_a_squadre/login.php";
const STATS_URL: &str = "https://www.phiquadro.it/gara_a_squadre/stampe/statistiche_squadra.php";
const CONTESTS_URL: &str = "https://www.phiquadro.it/gara_a_squadre/insegnanti_gestione_statistiche.php";

/// Fetches the data of a contest from phiquadro.it and inserts is into the database
pub async fn create_contest<'a>(
    mut db: Connection<DB>,
    phi: &PhiQuadroLogin,
    name: &str,
    id: i32,
    sess: i32,
    duration: u32,
    start_time: NaiveDateTime,
    drift: i32,
) -> crate::error::Result<'a, i32> {
    use self::schema::contests;

    info!("Adding contest {}/{}", id, sess);
    let mut client = get_phiquadro_client(phi)
        .await
        .context("While initializing PhiQuadro HTTP client")
        .attach_status(Status::ServiceUnavailable)?;

    let teams = get_teams(&mut client, id, sess)
        .await
        .context("While fetching teams for given contest")
        .attach_status(Status::ServiceUnavailable)?;
    info!("Teams are {:?}", &teams);

    return Ok(0);

    // Querying the amount of contests to get the new id
    let contest_id = contests::dsl::contests
        .select(count_star())
        .get_result::<i64>(&mut **db)
        .await? as i32;

    // Inserting the new contest
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

/// Creates a reqwest client already logged on PhiQuadro
pub async fn get_phiquadro_client(phi: &PhiQuadroLogin) -> anyhow::Result<Client> {
    let client = Client::builder()
        .cookie_provider(Arc::new(Jar::default()))
        .build()?;

    client
        .get(LOGIN_URL)
        .send()
        .await?
        .error_for_status()?;

    client
        .post(LOGIN_URL)
        .form(&[("user", &phi.username), ("password", &phi.password)])
        .send()
        .await?
        .error_for_status()?;

    Ok(client)
}

pub async fn get_teams(client: &mut Client, id_gara: i32, id_sess: i32) -> anyhow::Result<Vec<i32>> {
    // Right now forms only link to stats pages but this might change
    lazy_static! {
        static ref selector: Selector = Selector::parse("form > input:nth-child(3)").expect("Not a valid CSS selector");
    }

    let contest_html = client
        .post(CONTESTS_URL)
        .form(&[("id_gara", id_gara), ("id_sess", id_sess)])
        .send()
        .await?
        .error_for_status()?
        .text()
        .await?;

    Html::parse_document(&contest_html)
        .select(&selector)
        .map(|form| match form.attr("value") {
            Some(value) => Ok(value.parse()?),
            None => Err(anyhow!("PhiQuadro produced an input tag with no value")),
        })
        .collect::<anyhow::Result<Vec<i32>>>()
}
