DROP DATABASE IF EXISTS gas_simulator;
CREATE DATABASE gas_simulator;

\connect gas_simulator;

CREATE TABLE contests (
    id          INTEGER PRIMARY KEY,
    duration    INTERVAL,
    drift       INTEGER,

    CONSTRAINT positive_duration CHECK (duration >= '0'::INTERVAL),
    CONSTRAINT positive_drift CHECK (drift >= 0)
);

CREATE TABLE questions(
    id          INTEGER PRIMARY KEY,
    answer      INTEGER,
    contestId   INTEGER REFERENCES contests(id)
);

CREATE INDEX ON questions (contestId);

CREATE TABLE teams(
    id          INTEGER PRIMARY KEY,
    teamName    VARCHAR(255),
    isFake      BOOLEAN,
    contestId   INTEGER REFERENCES contests(id)
);

CREATE INDEX ON teams (contestId);

CREATE TABLE submissions(
    id          INTEGER PRIMARY KEY,
    answer      INTEGER,
    subTime     INTERVAL,
    questionId  INTEGER REFERENCES questions(id),

    CONSTRAINT positive_subTime CHECK (subTime >= '0'::INTERVAL)
);

CREATE INDEX ON submissions (questionId);

CREATE TABLE jollies (
    id          INTEGER PRIMARY KEY,
    subTime     INTERVAL,
    questionId  INTEGER REFERENCES questions(id),

    CONSTRAINT positive_subTime CHECK (subTime >= '0'::INTERVAL)
);

CREATE INDEX ON jollies (questionId);
