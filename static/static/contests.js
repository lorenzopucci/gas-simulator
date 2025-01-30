async function delete_contest(id) {
    confirm("Stai per cancellare questa gara! Sei sicuro?");

    fetch(`contest/${id}`, {
        method: "DELETE",
    }).then(response => {
        window.location.reload();
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
