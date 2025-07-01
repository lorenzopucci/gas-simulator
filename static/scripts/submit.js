window.onload = () => {
    load_header();

    setup_form(
        "submit",
        (data) => {
            return {
                "team_id": parseInt(data.get("team_id")),
                "question_id": parseInt(data.get("question_id")),
                "answer": parseInt(data.get("answer")),
            };
        },
        (response) => {
            if (response.status == 201) {
                response.json().then(body => {
                    if (body.correct) {
                        alert("Risposta esatta!");
                    } else {
                        alert("Risposta errata!");
                    }
                });
            } else {
                response.json().then(body => {
                    alert(body.error)
                });
            }
        },
    );
};

function submit_jolly(contest_id) {
    const form = document.getElementById("submit");
    const form_data = new FormData(form);

    fetch(`/api/contests/${contest_id}/jollies`, {
        method: "POST",
        body: JSON.stringify({
            "team_id": parseInt(form_data.get("team_id")),
            "question_id": parseInt(form_data.get("question_id")),
        }),
        headers: {
            "Content-Type": "application/json",
        }
    }).then((response) => {
        if (response.status == 201) {
            alert("Jolly scelto!");
        } else {
            response.json().then(body => {
                alert(body.error)
            });
        }
    });
}
