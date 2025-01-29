async function delete_contest(id) {
    confirm("Stai per cancellare questa gara! Sei sicuro?");

    fetch(`contest/${id}`, {
        method: "DELETE",
    }).then(response => {
        if (response.redirected) {
            window.location.replace(response.url);
        }
    })
}

function copy_contest_link(id) {
    navigator.clipboard.writeText(`${window.location.origin}/contest/${id}`);
}

function redirect_to_contest(id) {
    window.location.href = `contest/${id}`;
}
