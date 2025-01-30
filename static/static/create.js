window.onload = () => {
    setup_form(
        "create",
        (data) => {
            return {
                "phi_id": parseInt(data.get("phi_id")),
                "phi_sess": parseInt(data.get("phi_sess")),
                "name": data.get("name"),
                "start_time": data.get("start_time"),
                "duration": parseInt(data.get("duration")),
                "drift": parseInt(data.get("drift")),
                "drift_time": parseInt(data.get("drift_time")),
            };
        },
        (response) => {
            if (response.redirected) {
                window.location.replace(response.url);
            }
        },
    );
};
