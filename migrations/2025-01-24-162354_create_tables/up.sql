CREATE TABLE contests (
    id              INTEGER PRIMARY KEY NOT NULL GENERATED ALWAYS AS IDENTITY,
    phiquadro_id    INTEGER NOT NULL,
    phiquadro_sess  INTEGER NOT NULL,
    contest_name    VARCHAR(255) NOT NULL,
    duration        INTERVAL NOT NULL,
    start_time      TIMESTAMP NOT NULL,
    drift           INTEGER NOT NULL,
    drift_time      INTERVAL NOT NULL,

    CONSTRAINT positive_id CHECK (id >= 0),
    CONSTRAINT positive_duration CHECK (duration >= '0'::INTERVAL),
    CONSTRAINT positive_drift CHECK (drift >= 0)
);

CREATE INDEX on contests (id);

CREATE TABLE questions(
    id              INTEGER PRIMARY KEY NOT NULL GENERATED ALWAYS AS IDENTITY,
    answer          INTEGER NOT NULL,
    position        INTEGER NOT NULL,
    contest_id      INTEGER NOT NULL REFERENCES contests(id),

    UNIQUE (contest_id, position),
    CONSTRAINT positive_id CHECK (id >= 0),
    CONSTRAINT positive_position CHECK (position >= 0)
);

CREATE INDEX ON questions (contest_id);

CREATE TABLE teams(
    id              INTEGER PRIMARY KEY NOT NULL GENERATED ALWAYS AS IDENTITY,
    team_name       VARCHAR(255) NOT NULL,
    is_fake         BOOLEAN NOT NULL,
    position        INTEGER NOT NULL,
    contest_id      INTEGER NOT NULL REFERENCES contests(id),

    UNIQUE (contest_id, position),
    CONSTRAINT positive_id CHECK (id >= 0),
    CONSTRAINT positive_position CHECK (position >= 0)
);

CREATE INDEX ON teams (contest_id);

CREATE TABLE submissions(
    id              INTEGER PRIMARY KEY NOT NULL GENERATED ALWAYS AS IDENTITY,
    answer          INTEGER NOT NULL,
    sub_time        INTERVAL NOT NULL,
    team_id         INTEGER NOT NULL REFERENCES teams(id),
    question_id     INTEGER NOT NULL REFERENCES questions(id),

    CONSTRAINT positive_id CHECK (id >= 0),
    CONSTRAINT positive_sub_time CHECK (sub_time >= '0'::INTERVAL)
);

CREATE INDEX ON submissions (team_id);

CREATE TABLE jollies (
    id              INTEGER PRIMARY KEY NOT NULL GENERATED ALWAYS AS IDENTITY,
    sub_time        INTERVAL NOT NULL,
    team_id         INTEGER NOT NULL REFERENCES teams(id),
    question_id     INTEGER NOT NULL REFERENCES questions(id),

    UNIQUE (team_id),
    CONSTRAINT positive_id CHECK (id >= 0),
    CONSTRAINT positive_sub_time CHECK (sub_time >= '0'::INTERVAL)
);

CREATE INDEX ON jollies (team_id);
