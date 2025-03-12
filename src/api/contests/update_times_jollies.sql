UPDATE jollies
    SET sub_time = sub_time + $1
FROM questions
    WHERE questions.id = jollies.question_id AND contest_id = $2
