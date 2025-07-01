window.onload = () => {
    load_header();
    setup_submitter();
}

function redirect_to_ranking(id) {
    window.location.href = `/contest/${id}`;
}
