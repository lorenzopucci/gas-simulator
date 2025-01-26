use std::io::Write;
use std::process::{Command, Stdio};
use std::str::from_utf8;
use std::sync::Arc;
use std::thread;

use anyhow::{anyhow, bail, Context};
use chrono::NaiveDateTime;
use lazy_static::lazy_static;
use regex::bytes::Regex;
use reqwest::cookie::Jar;
use reqwest::Client;

use rocket::http::Status;
use rocket_db_pools::diesel::prelude::RunQueryDsl;
use rocket_db_pools::Connection;

use ::diesel::data_types::PgInterval;
use ::diesel::dsl::count_star;
use ::diesel::query_dsl::methods::SelectDsl;
use scraper::{Html, Selector};

use crate::model::{Contest, Jolly, Question, Submission, Team};
use crate::{PhiQuadroLogin, DB};

use crate::error::{IntoStatusResult, Result};

const LOGIN_URL: &str = "https://www.phiquadro.it/gara_a_squadre/login.php";
const STATS_URL: &str = "https://www.phiquadro.it/gara_a_squadre/stampe/statistiche_squadra.php";
const CONTESTS_URL: &str = "https://www.phiquadro.it/gara_a_squadre/insegnanti_gestione_statistiche.php";

#[derive(Clone, Debug)]
struct TeamActivity {
    submissions: Vec<(i32, i32, i32)>,
    jolly: Option<i32>,
}

/// Fetches the data of a contest from phiquadro.it and inserts is into the database
pub async fn create_contest(
    db: &mut Connection<DB>,
    phi: &PhiQuadroLogin,
    name: &str,
    id: i32,
    sess: i32,
    duration: u32,
    start_time: NaiveDateTime,
    drift: i32,
    drift_time: u32,
) -> Result<i32> {
    use crate::schema::{contests, jollies, questions, submissions, teams};

    info!("Adding contest {}/{}", id, sess);

    // Querying the amount of contests to get the new id
    let contest_id = contests::dsl::contests
        .select(count_star())
        .get_result::<i64>(db)
        .await
        .attach_status(Status::InternalServerError)? as i32;

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
            drift_time: PgInterval::from_microseconds((drift_time as i64) * 1_000_000),
        })
        .execute(db)
        .await
        .attach_status(Status::InternalServerError)?;

    // Setting up a phiquadro client
    let mut client = get_phiquadro_client(phi)
        .await
        .context("While initializing PhiQuadro HTTP client")
        .attach_status(Status::ServiceUnavailable)?;

    // Fetching the teams from phiquadro
    let teams = get_teams(&mut client, id, sess)
        .await
        .context("While fetching teams for given contest")
        .attach_status(Status::ServiceUnavailable)?;

    // Querying the amount of teams to get the new id
    let mut teams_id = teams::dsl::teams
        .select(count_star())
        .get_result::<i64>(db)
        .await
        .attach_status(Status::InternalServerError)? as i32;

    // Inserting the teams into the database
    diesel::insert_into(teams::table)
        .values(
            teams
                .iter()
                .enumerate()
                .map(|(i, (_team_id, team_name))| Team {
                    id: teams_id + i as i32,
                    team_name: team_name.clone(),
                    is_fake: true,
                    position: i as i32,
                    contest_id,
                })
                .collect::<Vec<_>>(),
        )
        .execute(db)
        .await
        .attach_status(Status::InternalServerError)?;

    // Querying the amount of submissions to get the new link
    let mut questions_id = questions::dsl::questions
        .select(count_star())
        .get_result::<i64>(db)
        .await
        .attach_status(Status::InternalServerError)? as i32;

    let mut questions = vec![];

    for i in 0..21 {
        questions.push(questions_id + i);
        diesel::insert_into(questions::table)
            .values(&Question {
                id: questions_id + i,
                contest_id,
                answer: [199, 8679, 9216, 784, 25, 1125, 4131, 6656, 53, 35, 22, 2000, 2400, 119, 36, 578, 340, 450, 3, 404, 1037][i as usize],
                position: i,
            })
            .execute(db)
            .await
            .attach_status(Status::InternalServerError)?;
    }

    for (team_id, team_name) in teams {
        info!("Inserting {team_id} {team_name}");
        let submissions = get_submissions(&mut client, id, sess, team_id).await?;

        // Querying the amount of submissions to get the new id
        let mut submissions_id = submissions::dsl::submissions
            .select(count_star())
            .get_result::<i64>(db)
            .await
            .attach_status(Status::InternalServerError)? as i32;

        diesel::insert_into(submissions::table)
            .values(
                submissions
                    .submissions
                    .iter()
                    .enumerate()
                    .map(|(i, &(sub_time, answer, question))| Submission {
                        id: submissions_id + i as i32,
                        question_id: questions[question as usize],
                        team_id: teams_id,
                        sub_time: PgInterval::from_microseconds((sub_time as i64) * 60_000_000),
                        answer,
                    })
                    .collect::<Vec<_>>(),
            )
            .execute(db)
            .await
            .attach_status(Status::InternalServerError)?;

        if let Some(jolly) = submissions.jolly {
            // Querying the amount of jollies to get the new id
            let mut jollies_id = jollies::dsl::jollies
                .select(count_star())
                .get_result::<i64>(db)
                .await
                .attach_status(Status::InternalServerError)? as i32;

            diesel::insert_into(jollies::table)
                .values(&Jolly {
                    id: jollies_id,
                    question_id: questions[jolly as usize],
                    sub_time: PgInterval::from_microseconds(600_000_000),
                    team_id: teams_id,
                })
                .execute(db)
                .await
                .attach_status(Status::InternalServerError)?;
        }

        teams_id += 1;
    }

    Ok(contest_id)
}

/// Creates a reqwest client already logged on PhiQuadro
async fn get_phiquadro_client(phi: &PhiQuadroLogin) -> anyhow::Result<Client> {
    let client = Client::builder().cookie_provider(Arc::new(Jar::default())).build()?;

    client.get(LOGIN_URL).send().await?.error_for_status()?;

    client
        .post(LOGIN_URL)
        .form(&[("user", &phi.username), ("password", &phi.password)])
        .send()
        .await?
        .error_for_status()?;

    Ok(client)
}

async fn get_teams(client: &mut Client, id_gara: i32, id_sess: i32) -> anyhow::Result<Vec<(i32, String)>> {
    // Right now forms only link to stats pages but this might change
    lazy_static! {
        static ref id_selector: Selector =
            Selector::parse("form > input:nth-child(3)").expect("not a valid CSS selector");
        static ref name_selector: Selector =
            Selector::parse("tr > td.cornice:nth-child(4)").expect("not a valid CSS selector");
    }

    let contest_html = client
        .post(CONTESTS_URL)
        .form(&[("id_gara", id_gara), ("id_sess", id_sess)])
        .send()
        .await?
        .error_for_status()?
        .text()
        .await?;

    let dom = Html::parse_document(&contest_html);
    let ids = dom.select(&id_selector);
    let names = dom.select(&name_selector);

    ids.zip(names)
        .map(|(id_form, name_td)| {
            let id = match id_form.attr("value") {
                Some(value) => value.parse()?,
                None => bail!("PhiQuadro produced an input tag with no value"),
            };

            Ok((id, name_td.text().collect()))
        })
        .collect()
}

async fn get_submissions(client: &mut Client, id_gara: i32, id_sess: i32, id_squadra: i32) -> Result<TeamActivity> {
    let log_pdf = client
        .post(STATS_URL)
        .form(&[("id_gara", id_gara), ("id_sess", id_sess), ("id_squadra", id_squadra)])
        .send()
        .await
        .attach_status(Status::ServiceUnavailable)?
        .error_for_status()
        .attach_status(Status::ServiceUnavailable)?
        .bytes()
        .await
        .attach_status(Status::ServiceUnavailable)?;

    let mut parse_pdf = Command::new("pdftotext")
        .arg("-layout")
        .arg("-nopgbrk")
        .arg("-")
        .arg("-")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .attach_status(Status::InternalServerError)?;

    let mut stdin = parse_pdf
        .stdin
        .take()
        .ok_or_else(|| anyhow!("failed to get stdin"))
        .attach_status(Status::InternalServerError)?;

    thread::spawn(move || stdin.write_all(&log_pdf).expect("failed to write to stdin"));

    let output = parse_pdf
        .wait_with_output()
        .attach_status(Status::InternalServerError)?
        .stdout;

    parse_team_text(&output).attach_status(Status::InternalServerError)
}

fn parse_team_text(text: &[u8]) -> anyhow::Result<TeamActivity> {
    lazy_static! {
        static ref parse_re: Regex = Regex::new(r"(DOMANDA)|(\(jolly\))|(?:dopo: (\d+) minuti +(?:[-+]\d+)?) +(\d+)")
            .expect("not a valid regex");
    }

    let mut curr = -1;
    let mut submissions = vec![];
    let mut jolly = None;

    for m in parse_re.captures_iter(text) {
        if m.get(1).is_some() {
            curr += 1;
        } else if m.get(2).is_some() {
            jolly = Some(curr);
        } else {
            let time = m
                .get(3)
                .ok_or_else(|| anyhow!("regex failed to find time of submission"))?;
            let time: i32 = from_utf8(time.as_bytes())?.parse()?;

            let answer = m
                .get(4)
                .ok_or_else(|| anyhow!("regex failed to find answer of submission"))?;
            let answer: i32 = from_utf8(answer.as_bytes())?.parse()?;

            submissions.push((time, answer, curr));
        }
    }

    Ok(TeamActivity { submissions, jolly })
}
