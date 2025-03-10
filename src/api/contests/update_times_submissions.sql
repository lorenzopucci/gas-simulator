UPDATE submissions
    SET sub_time = sub_time + $1
FROM questions
    WHERE contest_id = $2
