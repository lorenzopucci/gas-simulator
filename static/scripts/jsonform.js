function send_form(form, conv, callback) {
    const form_data = new FormData(form);

    fetch(form.getAttribute("action"), {
        method: form.getAttribute("method"),
        body: JSON.stringify(conv(form_data)),
        headers: {
            "Content-Type": "application/json",
        }
    }).then(callback);
}

function setup_form(id, conv, callback) {
    var form = document.getElementById(id);

    if (!form)
        return;

    form.onsubmit = (event) => {
        send_form(form, conv, callback);

        return false;
    };
}
