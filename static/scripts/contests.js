async function delete_contest(id) {
    if (!confirm("Stai per cancellare questa gara! Sei sicuro?")) {
        return;
    }

    fetch(`api/contests/${id}`, {
        method: "DELETE",
    }).then(response => {
        if (response.status == 204) {
            window.location.reload();
        } else {
            response.json().then(body => {
                alert(body.error)
            });
        }
    })
}

function copy_contest_link(id) {
    navigator.clipboard.writeText(`${window.location.origin}/contest/${id}`);
}

function redirect_to_contest(id) {
    window.location.href = `contest/${id}`;
}

function redirect_to_settings(id) {
    window.location.href = `settings/${id}`;
}
