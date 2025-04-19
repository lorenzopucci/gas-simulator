window.onload = () => {
    load_header();
    
    setup_form(
        "submit-answer",
        (data) => {
            return {
                "team_id": parseInt(data.get("team_id")),
                "question_id": parseInt(data.get("question_id")),
                "answer": parseInt(data.get("answer")),
            };
        },
        (response) => {
            if (response.status == 201) {
                response.json().then(body => {
                    if (body.correct) {
                        alert("Risposta esatta!");
                    } else {
                        alert("Risposta errata!");
                    }
                    reload_content();
                    hide_submitter();
                });
            } else {
                response.json().then(body => {
                    alert(body.error)
                });
            }
        },
    );

    setup_form(
        "submit-jolly",
        (data) => {
            return {
                "team_id": parseInt(data.get("team_id")),
                "question_id": parseInt(data.get("question_id")),
            };
        },
        (response) => {
            if (response.status == 201) {
                alert("Jolly scelto!");
                reload_content();
                hide_submitter();
            } else {
                response.json().then(body => {
                    alert(body.error)
                });
            }
        },
    );
};

setInterval(reload_content, 60000)

function reload_content() {
    const hidden_teams = document.getElementById("toggle-visibility").getAttribute("onclick") == "show_fake_teams()";

    fetch(window.location.href).then(body => body.text()).then(text => {
        const parser = new DOMParser();
        const doc = parser.parseFromString(text, "text/html");

        const ranking = doc.getElementById("ranking");

        document.getElementById("ranking").innerHTML = ranking.innerHTML;

        if (hidden_teams) {
            hide_fake_teams();
        }
    })
}

function hide_fake_teams() {
    Array.from(document.getElementsByClassName("fake-team")).forEach(elem => {
        elem.setAttribute("hidden", "");
    });

    let toggler = document.getElementById("toggle-visibility")
    toggler.setAttribute('onclick', "show_fake_teams()");
    toggler.innerText = "Mostra squadre fantasma";
}

function show_fake_teams() {
    Array.from(document.getElementsByClassName("fake-team")).forEach(elem => {
        elem.removeAttribute("hidden");
    });

    let toggler = document.getElementById("toggle-visibility")
    toggler.setAttribute('onclick', "hide_fake_teams()");
    toggler.innerText = "Nascondi squadre fantasma";
}

function show_submitter() {
    document.getElementById("submitter-background").style.visibility = "visible";
    document.getElementById("submitter").style.visibility = "visible";
}

function hide_submitter() {
    document.getElementById("submitter-background").style.visibility = "hidden";
    document.getElementById("submitter").style.visibility = "hidden";
}
