#![allow(clippy::get_first)]

use std::cmp;

use diesel::dsl::count_star;
use diesel::{ExpressionMethods, QueryDsl};
use rocket_db_pools::diesel::prelude::RunQueryDsl;
use rocket_db_pools::Connection;
use tracing::info;

use super::contest::{Contest, Question, Team};
use crate::contest::contest::{QuestionStatus, TeamQuestion};
use crate::model::{self, ContestJollies, ContestSubmissions};
use crate::model::get_duration;
use crate::DB;

pub async fn fetch_contest(db: &mut Connection<DB>, id: i32) -> anyhow::Result<Option<Contest>> {
    use crate::schema::{contests, jollies, submissions, questions, teams};

    info!("Loading contest {}", id);

    let contest = contests::dsl::contests
        .filter(contests::dsl::id.eq(id))
        .load::<model::Contest>(db)
        .await?;

    let Some(contest) = contest.get(0) else {
        return Ok(None);
    };

    let teams = teams::dsl::teams
        .filter(teams::contest_id.eq(id))
        .load::<model::Team>(db)
        .await?;

    let questions_no = questions::dsl::questions
        .select(count_star())
        .filter(questions::contest_id.eq(id))
        .load::<i64>(db)
        .await?[0] as usize;

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
        score: 0,
        questions: vec![TeamQuestion::default(); questions_no],
    }).collect();

    let mut questions: Vec<Question> = (0..questions_no).map(|idx| Question {
        position: idx as i32,
        score: 20,
        locked: false,
    }).collect();

    let mut drift_left = vec![contest.drift; questions_no];
    let mut drift = vec![get_duration(contest.drift_time); questions_no];

    for submission in &submissions {
        if submission.given_answer == submission.correct_answer {
            drift_left[submission.question_pos as usize] -= 1;
            if drift_left[submission.question_pos as usize] <= 0 {
                drift[submission.question_pos as usize] = cmp::min(drift[submission.question_pos as usize], get_duration(submission.sub_time));
                questions[submission.question_pos as usize].locked = true;
            }
        } else {
            if get_duration(submission.sub_time) < drift[submission.question_pos as usize] {
                questions[submission.question_pos as usize].score += 2;
            }
        }
    }

    for i in 0..questions_no {
        questions[i].score += drift[i].num_minutes() as i32;
    }

    for submission in &submissions {
        if submission.given_answer == submission.correct_answer {
            if teams[submission.team_pos as usize].questions[submission.question_pos as usize].status != QuestionStatus::Solved {
                teams[submission.team_pos as usize].questions[submission.question_pos as usize].score += questions[submission.question_pos as usize].score;
            }
            teams[submission.team_pos as usize].questions[submission.question_pos as usize].status = QuestionStatus::Solved;
        } else {
            if teams[submission.team_pos as usize].questions[submission.question_pos as usize].status == QuestionStatus::NotAttempted {
                teams[submission.team_pos as usize].questions[submission.question_pos as usize].status = QuestionStatus::Attempted;
            }
            teams[submission.team_pos as usize].questions[submission.question_pos as usize].score -= 10;
        }
    }

    for jolly in jollies {
        teams[jolly.team_pos as usize].questions[jolly.question_pos as usize].score *= 2;
        teams[jolly.team_pos as usize].questions[jolly.question_pos as usize].jolly = true;
    }

    for team in &mut teams {
        team.score = questions_no as i32 * 10 + team.questions.iter().map(|q| q.score).sum::<i32>();
    }

    teams.sort_unstable_by_key(|team| -team.score);

    Ok(Some(Contest {
        name: contest.contest_name.clone(),
        phi_id: contest.phiquadro_id,
        phi_sess: contest.phiquadro_sess,
        duration: get_duration(contest.duration),
        drift: contest.drift,
        start_time: contest.start_time.and_utc(),
        questions,
        teams,
    }))
}
