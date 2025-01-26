use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Serialize, Deserialize)]
pub struct Question {
    pub score: i64,
    pub locked: bool,
}

#[derive(Clone, PartialEq, Eq, Copy, Default, Serialize, Deserialize)]
pub enum QuestionStatus {
    #[default]
    NotAttempted,
    Attempted,
    JustSolved,
    Solved,
}

#[derive(Clone, Copy, Default, Serialize, Deserialize)]
pub struct TeamQuestion {
    pub score: i64,
    pub jolly: bool,
    pub status: QuestionStatus,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Team {
    pub name: String,
    pub is_fake: bool,
    pub score: i64,
    pub questions: Vec<TeamQuestion>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Contest {
    pub name: String,
    pub phi_id: i32,
    pub phi_sess: i32,
    pub questions: Vec<Question>,
    pub teams: Vec<Team>,
    pub duration: Duration,
    pub start_time: DateTime<Utc>,
    #[serde(default = "default_drift")]
    pub drift: i32,
}

const fn default_drift() -> i32 {
    3
}
