function hide_fake_teams() {
    Array.from(document.getElementsByClassName("fake-team")).forEach(elem => {
        elem.style.visibility = "hidden";
    });

    let toggler = document.getElementById("toggle-visibility")
    toggler.setAttribute('onclick', "show_fake_teams()");
    toggler.innerText = "Mostra squadre fantasma";
}

function show_fake_teams() {
    Array.from(document.getElementsByClassName("fake-team")).forEach(elem => {
        elem.style.visibility = "visible";
    });

    let toggler = document.getElementById("toggle-visibility")
    toggler.setAttribute('onclick', "hide_fake_teams()");
    toggler.innerText = "Nascondi squadre fantasma";
}
