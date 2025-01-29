#![allow(clippy::get_first)]

use std::cmp;

use chrono::TimeDelta;
use diesel::{ExpressionMethods, QueryDsl};
use rocket_db_pools::diesel::prelude::RunQueryDsl;
use rocket_db_pools::Connection;
use tracing::info;

use super::contest::{Contest, Question, Team};
use crate::contest::contest::{QuestionStatus, TeamQuestion};
use crate::model::{self, ContestJollies, ContestSubmissions};

use crate::DB;

const QUESTION_BONUS: [i64; 10] = [20, 15, 10, 8, 6, 5, 4, 3, 2, 1];
const CONTEST_BONUS: [i64; 6] = [100, 60, 40, 30, 20, 10];

pub async fn fetch_contest(db: &mut Connection<DB>, id: i32) -> anyhow::Result<Option<Contest>> {
    use crate::schema::{contests, jollies, submissions, questions, teams};

    info!("Loading contest {}", id);

    let contest = contests::dsl::contests
        .select((
            contests::phiquadro_id,
            contests::phiquadro_sess,
            contests::contest_name,
            contests::duration,
            contests::start_time,
            contests::drift,
            contests::drift_time,
            contests::teams_no,
            contests::questions_no,
            contests::active,
        ))
        .filter(contests::dsl::id.eq(id))
        .filter(contests::active.eq(true))
        .load::<model::Contest>(db)
        .await?;

    let Some(contest) = contest.get(0) else {
        return Ok(None);
    };

    let questions_no = contest.questions_no as usize;

    let teams = teams::dsl::teams
        .select((
            teams::team_name,
            teams::is_fake,
            teams::position,
            teams::contest_id,
        ))
        .filter(teams::contest_id.eq(id))
        .load::<model::Team>(db)
        .await?;

    let submissions = submissions::dsl::submissions
        .inner_join(questions::table)
        .inner_join(teams::table)
        .select((
            submissions::answer,
            submissions::sub_time,
            questions::answer,
            questions::position,
            teams::position,
            teams::is_fake,
            teams::contest_id
        ))
        .filter(teams::contest_id.eq(id))
        .order(submissions::sub_time.asc())
        .load::<ContestSubmissions>(db)
        .await?;

    let jollies = jollies::dsl::jollies
        .inner_join(questions::table)
        .inner_join(teams::table)
        .select((
            jollies::sub_time,
            questions::position,
            teams::position,
            teams::contest_id,
        ))
        .filter(teams::contest_id.eq(id))
        .load::<ContestJollies>(db)
        .await?;

    let mut teams: Vec<Team> = teams.iter().map(|team| Team {
        name: team.team_name.clone(),
        is_fake: team.is_fake,
        score: questions_no as i64 * 10,
        questions: vec![TeamQuestion::default(); questions_no],
    }).collect();

    let mut questions: Vec<Question> = vec![Question {
        score: 20,
        locked: false,
    }; questions_no];

    let mut drift_left = vec![contest.drift; questions_no];
    let mut wrong = vec![vec![false; questions_no]; teams.len()];
    let mut drift = vec![TimeDelta::seconds(contest.drift_time as i64); questions_no];

    for submission in &submissions {
        let q_pos = submission.question_pos as usize;
        let t_pos = submission.team_pos as usize;
        let sub_time = TimeDelta::seconds(submission.sub_time as i64);

        if submission.given_answer == submission.correct_answer {
            drift_left[q_pos] -= 1;
            if drift_left[q_pos] <= 0 {
                drift[q_pos] = cmp::min(drift[q_pos], sub_time);
                questions[q_pos].locked = true;
            }
        } else {
            if sub_time < drift[q_pos] && !wrong[t_pos][q_pos] {
                questions[q_pos].score += 2;
            }
            wrong[t_pos][q_pos] = true;
        }
    }

    for i in 0..questions_no {
        questions[i].score += drift[i].num_minutes();
    }

    let mut question_solves = vec![0; questions_no];
    let mut team_solves = vec![0; teams.len()];
    let mut solves = 0;

    for submission in &submissions {
        let q_pos = submission.question_pos as usize;
        let t_pos = submission.team_pos as usize;

        if submission.given_answer == submission.correct_answer {
            if teams[t_pos].questions[q_pos].status != QuestionStatus::Solved {
                teams[t_pos].questions[q_pos].score += questions[q_pos].score + QUESTION_BONUS.get(question_solves[q_pos]).unwrap_or(&0);

                question_solves[q_pos] += 1;
                team_solves[t_pos] += 1;

                if team_solves[t_pos] == questions_no {
                    teams[t_pos].score += CONTEST_BONUS.get(solves).unwrap_or(&0);
                    solves += 1;
                }
            }
            teams[t_pos].questions[q_pos].status = QuestionStatus::Solved;
        } else {
            if teams[t_pos].questions[q_pos].status == QuestionStatus::NotAttempted {
                teams[t_pos].questions[q_pos].status = QuestionStatus::Attempted;
            }
            teams[t_pos].questions[q_pos].score -= 10;
        }
    }

    for jolly in jollies {
        teams[jolly.team_pos as usize].questions[jolly.question_pos as usize].score *= 2;
        teams[jolly.team_pos as usize].questions[jolly.question_pos as usize].jolly = true;
    }

    for team in &mut teams {
        team.score += team.questions.iter().map(|q| q.score).sum::<i64>();
    }

    teams.sort_unstable_by_key(|team| -team.score);

    Ok(Some(Contest {
        name: contest.contest_name.clone(),
        phi_id: contest.phiquadro_id,
        phi_sess: contest.phiquadro_sess,
        duration: TimeDelta::seconds(contest.duration as i64),
        drift: contest.drift,
        start_time: contest.start_time.and_utc(),
        questions,
        teams,
    }))
}
