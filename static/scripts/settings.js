window.onload = () => {
    setup_form(
        "update",
        (data) => {
            return {
                "start_time": data.get("start_time"),
                "duration": parseInt(data.get("duration")),
                "drift": parseInt(data.get("drift")),
                "drift_time": parseInt(data.get("drift_time")),
            };
        },
        (response) => {
            if (response.status == 204) {
                window.location.reload();
            } else {
                response.json().then(body => {
                    alert(body.error)
                });
            }
        },
    );
};

async function delete_team(contest_id, id) {
    if (!confirm("Stai per cancellare questa squadra! Sei sicuro?")) {
        return;
    }

    fetch(`/api/contests/${contest_id}/teams/${id}`, {
        method: "DELETE",
    }).then(response => {
        if (response.status == 204) {
            window.location.reload();
        }
    })
}

function reload_callback(response) {
    console.log(response);
    if (response.status == 201) {
        window.location.reload();
    }
}

function conv_add_team(data) {
    return {
        "team_name": data.get("team_name"),
    };
}
