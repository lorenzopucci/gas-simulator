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
            if (response.status == 202) {
                window.location.reload();
            }
        },
    );
};

async function delete_team(id) {
    confirm("Stai per cancellare questa squadra! Sei sicuro?");

    fetch(`/teams/${id}`, {
        method: "DELETE",
    }).then(response => {
        if (response.status == 202) {
            window.location.reload();
        }
    })
}

function reload_callback(response) {
    console.log(response);
    if (response.status == 202) {
        window.location.reload();
    }
}

function conv_add_team(data) {
    return {
        "team_name": data.get("team_name"),
    };
}
