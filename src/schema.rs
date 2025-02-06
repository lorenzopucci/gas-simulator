// @generated automatically by Diesel CLI.

diesel::table! {
    contests (id) {
        id -> Int4,
        phiquadro_id -> Int4,
        phiquadro_sess -> Int4,
        #[max_length = 255]
        contest_name -> Varchar,
        duration -> Int4,
        start_time -> Timestamptz,
        drift -> Int4,
        drift_time -> Int4,
        teams_no -> Int4,
        questions_no -> Int4,
        active -> Bool,
        owner_id -> Int4,
    }
}

diesel::table! {
    jollies (id) {
        id -> Int4,
        sub_time -> Timestamptz,
        team_id -> Int4,
        question_id -> Int4,
    }
}

diesel::table! {
    questions (id) {
        id -> Int4,
        answer -> Int4,
        position -> Int4,
        contest_id -> Int4,
    }
}

diesel::table! {
    submissions (id) {
        id -> Int4,
        answer -> Int4,
        sub_time -> Timestamptz,
        team_id -> Int4,
        question_id -> Int4,
    }
}

diesel::table! {
    teams (id) {
        id -> Int4,
        #[max_length = 255]
        team_name -> Varchar,
        is_fake -> Bool,
        position -> Int4,
        contest_id -> Int4,
    }
}

diesel::table! {
    tokens (id) {
        id -> Int4,
        user_id -> Int4,
        #[max_length = 344]
        token -> Bpchar,
        expires -> Timestamptz,
    }
}

diesel::table! {
    users (id) {
        id -> Int4,
        #[max_length = 255]
        username -> Varchar,
        #[max_length = 255]
        email -> Varchar,
        password_hash -> Bytea,
        salt -> Bytea,
    }
}

diesel::joinable!(contests -> users (owner_id));
diesel::joinable!(jollies -> questions (question_id));
diesel::joinable!(jollies -> teams (team_id));
diesel::joinable!(questions -> contests (contest_id));
diesel::joinable!(submissions -> questions (question_id));
diesel::joinable!(submissions -> teams (team_id));
diesel::joinable!(teams -> contests (contest_id));
diesel::joinable!(tokens -> users (user_id));

diesel::allow_tables_to_appear_in_same_query!(
    contests,
    jollies,
    questions,
    submissions,
    teams,
    tokens,
    users,
);
