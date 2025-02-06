window.onload = () => {
    setup_form(
        "create",
        (data) => {
            return {
                "phiquadro_id": parseInt(data.get("phiquadro_id")),
                "phiquadro_sess": parseInt(data.get("phiquadro_sess")),
                "name": data.get("name"),
                "start_time": data.get("start_time"),
                "duration": parseInt(data.get("duration")),
                "drift": parseInt(data.get("drift")),
                "drift_time": parseInt(data.get("drift_time")),
            };
        },
        (response) => {
            if (response.status == 201) {
                response.json().then((body) => {
                    history.pushState({}, "");
                    window.location.replace(`contest/${body.contest_id}`);
                });
            } else {
                response.json().then(body => {
                    alert(body.error)
                });
            }
        },
    );
};
