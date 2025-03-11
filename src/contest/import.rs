use std::io::Write;
use std::process::{Command, Stdio};
use std::str::from_utf8;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use anyhow::{anyhow, bail, Context};
use chrono::{DateTime, TimeDelta, Utc};
use diesel::{update, ExpressionMethods, QueryDsl};
use lazy_static::lazy_static;
use regex::bytes::Regex;
use reqwest::cookie::Jar;
use reqwest::Client;

use rocket::http::hyper::body::Bytes;
use rocket::http::Status;
use rocket_db_pools::diesel::prelude::RunQueryDsl;
use rocket_db_pools::Connection;

use scraper::{Html, Selector};

use crate::model::{Contest, Jolly, Question, Submission, Team};
use crate::{PhiQuadroLogin, DB};

use crate::error::{IntoStatusResult, Result};

const LOGIN_URL: &str = "https://www.phiquadro.it/gara_a_squadre/login.php";
const CONTEST_STATS_URL: &str = "https://www.phiquadro.it/gara_a_squadre/stampe/statistiche_gara.php";
const TEAM_STATS_URL: &str = "https://www.phiquadro.it/gara_a_squadre/stampe/statistiche_squadra.php";
const CONTESTS_URL: &str = "https://www.phiquadro.it/gara_a_squadre/insegnanti_gestione_statistiche.php";

#[derive(Clone, Debug)]
struct TeamActivity {
    submissions: Vec<(i64, i32, usize)>,
    jolly: Option<usize>,
}

#[derive(Clone, Debug)]
struct ContestInfo {
    name: String,
    teams: Vec<(i32, String)>,
}

/// Fetches the data of a contest from phiquadro.it and inserts is into the database
pub async fn create_contest(
    db: &mut Connection<DB>,
    phi: &PhiQuadroLogin,
    owner_id: i32,
    name: &str,
    id: u32,
    sess: u32,
    duration: u32,
    start_time: DateTime<Utc>,
    drift: u32,
    drift_time: u32,
    jolly_time: u32,
    question_bonus: [i32; 10],
    contest_bonus: [i32; 10],
) -> Result<i32> {
    use crate::schema::{contests, jollies, questions, submissions, teams};

    info!("Adding contest {}/{}", id, sess);

    // Sanity checks of the values to insert

    let id = id
        .try_into()
        .map_err(|_| anyhow!("PhiQuadro ID should be a reasonable value ({} given)", id))
        .attach_info(Status::UnprocessableEntity, "ID PhiQuadro non valido")?;

    let sess = sess
        .try_into()
        .map_err(|_| anyhow!("PhiQuadro session should be a reasonable value ({} given)", sess))
        .attach_info(Status::UnprocessableEntity, "Sessione PhiQuadro non valida")?;

    let drift = drift
        .try_into()
        .map_err(|_| anyhow!("Drift should be a reasonable value ({} given)", drift))
        .attach_info(Status::UnprocessableEntity, "Deriva non valida")?;

    let duration = duration
        .try_into()
        .map_err(|_| anyhow!("Duration should be a reasonable value ({} given)", duration))
        .attach_info(Status::UnprocessableEntity, "Durata non valida")?;

    let drift_time = drift_time
        .try_into()
        .map_err(|_| anyhow!("Drift time should be a reasonable value ({} given)", drift_time))
        .attach_info(Status::UnprocessableEntity, "Durata deriva non valida")?;

    let jolly_time = jolly_time
        .try_into()
        .map_err(|_| anyhow!("Jolly time should be a reasonable value ({} given)", jolly_time))
        .attach_info(Status::UnprocessableEntity, "Durata scelta jolly non valida")?;

    let question_bonus = question_bonus
        .into_iter()
        .map(Some)
        .collect::<Vec<_>>();

    let contest_bonus = contest_bonus
        .into_iter()
        .map(Some)
        .collect::<Vec<_>>();

    // Setting up a phiquadro client
    let mut client = get_phiquadro_client(phi)
        .await
        .context("While initializing PhiQuadro HTTP client")
        .attach_info(Status::ServiceUnavailable, "Non riesco a contattare PhiQuadro")?;

    // Fetching the teams from phiquadro
    let contest_info = get_contest_info(&mut client, id, sess)
        .await
        .context("While fetching teams for given contest")
        .attach_info(Status::ServiceUnavailable, "Non riesco a importare la gara")?;

    let teams = contest_info.teams;
    let name = if name.is_empty() { &contest_info.name } else { name };

    // Fetching the questions from phiquadro
    let answers = get_questions(&mut client, id, sess).await?;
    info!("Answers are {:?}", answers);

    // Inserting the new contest
    let contest_id = diesel::insert_into(contests::table)
        .values(&Contest {
            contest_name: name.to_string(),
            phiquadro_id: id,
            phiquadro_sess: sess,
            duration,
            start_time,
            drift,
            drift_time,
            jolly_time,
            teams_no: teams.len() as i32,
            questions_no: answers.len() as i32,
            active: false,
            question_bonus,
            contest_bonus,
            owner_id: owner_id,
        })
        .returning(contests::id)
        .get_result(db)
        .await
        .attach_info(Status::InternalServerError, "Errore incontrato durante l'importazione della gara")?;

    // Inserting the teams into the database
    let teams_id = diesel::insert_into(teams::table)
        .values(
            teams
                .iter()
                .enumerate()
                .map(|(i, (_team_id, team_name))| Team {
                    team_name: team_name.clone(),
                    is_fake: true,
                    position: i as i32,
                    contest_id,
                })
                .collect::<Vec<_>>(),
        )
        .returning(teams::id)
        .get_results(db)
        .await
        .attach_info(Status::InternalServerError, "Errore incontrato durante l'importazione della gara")?;

    let questions = diesel::insert_into(questions::table)
        .values(
            answers
                .iter()
                .enumerate()
                .map(|(i, &answer)| Question {
                    contest_id,
                    position: i as i32,
                    answer,
                })
                .collect::<Vec<_>>(),
        )
        .returning(questions::id)
        .get_results(db)
        .await
        .attach_info(Status::InternalServerError, "Errore incontrato durante l'importazione della gara")?;

    for (i, (team_id, team_name)) in teams.iter().enumerate() {
        info!("Inserting {team_id} {team_name}");
        let submissions = get_submissions(&mut client, id, sess, *team_id).await?;

        diesel::insert_into(submissions::table)
            .values(
                submissions
                    .submissions
                    .iter()
                    .map(|&(sub_time, answer, question)| Submission {
                        question_id: questions[question],
                        team_id: teams_id[i],
                        sub_time: start_time + TimeDelta::minutes(sub_time),
                        answer,
                    })
                    .collect::<Vec<_>>(),
            )
            .execute(db)
            .await
            .attach_info(Status::InternalServerError, "Errore incontrato durante l'importazione della gara")?;

        if let Some(jolly) = submissions.jolly {
            diesel::insert_into(jollies::table)
                .values(&Jolly {
                    question_id: questions[jolly],
                    sub_time: start_time + TimeDelta::minutes(10),
                    team_id: teams_id[i],
                })
                .execute(db)
                .await
                .attach_info(Status::InternalServerError, "Errore incontrato durante l'importazione della gara")?;
        }
    }

    update(contests::dsl::contests.filter(contests::id.eq(contest_id)))
        .set(contests::active.eq(true))
        .execute(db)
        .await
        .attach_info(Status::InternalServerError, "Errore incontrato durante l'importazione della gara")?;

    Ok(contest_id)
}

/// Creates a reqwest client already logged on PhiQuadro
async fn get_phiquadro_client(phi: &PhiQuadroLogin) -> anyhow::Result<Client> {
    let client = Client::builder()
        .timeout(Duration::from_secs(10))
        .cookie_provider(Arc::new(Jar::default()))
        .build()?;

    client.get(LOGIN_URL).send().await?.error_for_status()?;

    client
        .post(LOGIN_URL)
        .form(&[("user", &phi.username), ("password", &phi.password)])
        .send()
        .await?
        .error_for_status()?;

    Ok(client)
}

/// Parses phiquadro html to find the teams taking part in a contest and the name of the contest
async fn get_contest_info(client: &mut Client, id_gara: i32, id_sess: i32) -> anyhow::Result<ContestInfo> {
    // Right now forms only link to stats pages but this might change
    lazy_static! {
        static ref id_selector: Selector =
            Selector::parse("form > input:nth-child(3)").expect("not a valid CSS selector");
        static ref name_selector: Selector =
            Selector::parse("tr > td.cornice:nth-child(4)").expect("not a valid CSS selector");
        static ref title_selector: Selector =
            Selector::parse("tr > td.titolo2:nth-child(3)").expect("not a valid CSS selector");
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

    let title = dom
        .select(&title_selector)
        .next()
        .ok_or_else(|| anyhow!("PhiQuadro produced a contest page with no contest title"))?
        .text()
        .collect();

    let ids = dom.select(&id_selector);
    let names = dom.select(&name_selector);

    let teams = ids
        .zip(names)
        .map(|(id_form, name_td)| {
            let id = match id_form.attr("value") {
                Some(value) => value.parse()?,
                None => bail!("PhiQuadro produced an input tag with no value"),
            };

            Ok((id, name_td.text().collect()))
        })
        .collect::<anyhow::Result<_>>()?;

    Ok(ContestInfo { name: title, teams })
}

/// Fetched the general pdf related to a contest
async fn get_questions(client: &mut Client, id_gara: i32, id_sess: i32) -> Result<Vec<i32>> {
    let log_pdf = client
        .post(CONTEST_STATS_URL)
        .form(&[("id_gara", id_gara), ("id_sess", id_sess)])
        .send()
        .await
        .attach_info(Status::ServiceUnavailable, "Non riesco a contattare PhiQuadro")?
        .error_for_status()
        .attach_info(Status::ServiceUnavailable, "Non riesco a contattare PhiQuadro")?
        .bytes()
        .await
        .attach_info(Status::ServiceUnavailable, "Non riesco a contattare PhiQuadro")?;

    let output = parse_pdf(log_pdf)
        .attach_info(Status::InternalServerError, "Errore incontrato durante l'importazione della gara")?;

    parse_contest_pdf(&output)
        .attach_info(Status::InternalServerError, "Errore incontrato durante l'importazione della gara")
}

/// Fetched the submission pdf related to a team
async fn get_submissions(client: &mut Client, id_gara: i32, id_sess: i32, id_squadra: i32) -> Result<TeamActivity> {
    let log_pdf = client
        .post(TEAM_STATS_URL)
        .form(&[("id_gara", id_gara), ("id_sess", id_sess), ("id_squadra", id_squadra)])
        .send()
        .await
        .attach_info(Status::ServiceUnavailable, "Non riesco a contattare PhiQuadro")?
        .error_for_status()
        .attach_info(Status::ServiceUnavailable, "Non riesco a contattare PhiQuadro")?
        .bytes()
        .await
        .attach_info(Status::ServiceUnavailable, "Non riesco a contattare PhiQuadro")?;

    let output = parse_pdf(log_pdf).
        attach_info(Status::InternalServerError, "Errore incontrato durante l'importazione della gara")?;

    parse_team_text(&output)
        .attach_info(Status::InternalServerError, "Errore incontrato durante l'importazione della gara")
}

/// Parses a pdf into plain text
fn parse_pdf(pdf: Bytes) -> anyhow::Result<Vec<u8>> {
    let mut parse_pdf = Command::new("pdftotext")
        .arg("-layout")
        .arg("-nopgbrk")
        .arg("-")
        .arg("-")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()?;

    let mut stdin = parse_pdf.stdin.take().ok_or_else(|| anyhow!("failed to get stdin"))?;

    thread::spawn(move || stdin.write_all(&pdf).expect("failed to write to stdin"));

    Ok(parse_pdf.wait_with_output()?.stdout)
}

/// Parses the pdf of a contest
fn parse_contest_pdf(text: &[u8]) -> anyhow::Result<Vec<i32>> {
    lazy_static! {
        static ref parse_re: Regex = Regex::new(r" *\d+ +(\d+)(?: +\d+){4,5}").expect("not a valid regex");
    }

    parse_re
        .captures_iter(text)
        .map(|caps| Ok(from_utf8(caps.extract::<1>().1[0])?.parse()?))
        .collect()
}

/// Parses the pdf of a team
fn parse_team_text(text: &[u8]) -> anyhow::Result<TeamActivity> {
    lazy_static! {
        static ref parse_re: Regex = Regex::new(r"(DOMANDA)|(\(jolly\))|(?:dopo: (\d+) minuti +(?:[-+]\d+)?) +(\d+)")
            .expect("not a valid regex");
    }

    let mut curr = 0;
    let mut submissions = vec![];
    let mut jolly = None;

    for m in parse_re.captures_iter(text) {
        if m.get(1).is_some() {
            curr += 1;
        } else if m.get(2).is_some() {
            jolly = Some(curr - 1);
        } else {
            let time = m
                .get(3)
                .ok_or_else(|| anyhow!("regex failed to find time of submission"))?;
            let time = from_utf8(time.as_bytes())?.parse()?;

            let answer = m
                .get(4)
                .ok_or_else(|| anyhow!("regex failed to find answer of submission"))?;
            let answer = from_utf8(answer.as_bytes())?.parse()?;

            submissions.push((time, answer, curr - 1));
        }
    }

    Ok(TeamActivity { submissions, jolly })
}
