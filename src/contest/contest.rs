use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Serialize, Deserialize)]
struct Question {
    id: i32,
    score: i32,
    locked: bool,
}

#[derive(Clone, Copy, Default, Serialize, Deserialize)]
enum QuestionStatus {
    #[default]
    NotAttempted,
    Attempted,
    JustSolved,
    Solved,
}

#[derive(Clone, Copy, Default, Serialize, Deserialize)]
struct TeamQuestion {
    score: i32,
    jolly: bool,
    status: QuestionStatus,
}

#[derive(Clone, Serialize, Deserialize)]
struct Team {
    name: String,
    is_fake: bool,
    score: i32,
    questions: Vec<TeamQuestion>,
}

#[derive(Clone, Serialize, Deserialize)]
struct Contest {
    questions: Vec<Question>,
    teams: Vec<Team>,
    duration: u32,
    #[serde(default = "default_drift")]
    drift: i32,
}

const fn default_drift() -> i32 {
    3
}
