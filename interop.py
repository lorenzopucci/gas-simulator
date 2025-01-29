import re
import requests
import sched
import string
import subprocess
import time
from datetime import datetime, timedelta

COOKIE = 'PHPSESSID=v2qcjc7pfpboea86b6m4nmegkj; phisession=y2XHc2aOGUwqsH5cN8oJmlTm3O3AOxpx5lrea4s1F6Jw4C5r'
URL_GARE = 'https://www.phiquadro.it/gara_a_squadre/insegnanti_gestione_statistiche.php'
URL_STATS = 'https://www.phiquadro.it/gara_a_squadre/stampe/statistiche_squadra.php'
URL_GESTIONE_SQUADRE = 'https://www.phiquadro.it/gara_a_squadre/insegnanti_gestione_squadre.php'
URL_INSERIMENTO_DOMANDE = 'https://www.phiquadro.it/gara_a_squadre/insegnanti_gestione_domande.php'
URL_ESEGUI = 'https://www.phiquadro.it/gara_a_squadre/esegui.php'
URL_INIZA = 'https://www.phiquadro.it/gara_a_squadre/inizia_gara.php'

TEAM_REGEX = '<td class=\'cornice\'><a href=\'#\' onclick="document\\.stat_sq_(\\d+)\\.submit\\(\\)" ><img src=\'..\\/immagini\\/stampa\\.png\' border=\'0\' alt=\'\'><\\/a><\\/td>\\n\\t{7}<td class=\'cornice\'>(.*)</td>'
NEW_TEAM_REGEX = '<a href=\'#\' onclick="document\\.delete_(\\d+).submit\\(\\)" ><img src=\'\\.\\.\\/immagini\\/delete\\.png\' border=\'0\' alt=\'Elimina\'><\\/a><\\/td>\\n\\t{7}<td class=\'cornice\'> %s <\\/td>'
STATS_REGEX = '(DOMANDA)|(\\(jolly\\))|(?:dopo: (\\d+) minuti +(?:[-+]\\d+)?) +(\\d+)'
ANSWER_REGEX = ''
QUESTION_REGEX = '<input type=\'hidden\' name=\'id_dom_mod\' value=\'(\\d+)\'>'

def filter_non_printable_chars(str):
    return ''.join(filter(lambda x: x in string.printable, str))

def start_gara(id_nuova_gara):
    req = session.post(URL_INIZA, data = {'id_gara': id_nuova_gara, 'INIZIA': 1})
    print(req.text)

def set_jolly(id_gara, id_sess, id_squadra, id_task):
    print(f"{id_squadra} sets {id_task} as jolly")
    req = session.post(URL_ESEGUI, data = {
        'id_squadra': id_squadra,
        'jolly': id_task,
        'INS_JOLLY': 1,
    })

    print(req)

def submit(id_gara, id_sess, id_squadra, id_task, answer):
    print(f"{id_squadra} submits {answer} to {id_task}")

    time = datetime.now()
    req = session.post(URL_ESEGUI, data = {
        'risposta': answer,
        'tempo_h': time.hour,
        'tempo_m': time.minute,
        'tempo_s': time.second,
        'id_squadra': id_squadra,
        'id_domanda': id_task,
        'id_gara': id_gara,
        'id_sess': id_sess,
        'ospite': 0,
        'INS': 1,
    })

    print(req)

def schedule_team_submissions(name, actions, id_nuova_gara, id_nuovo_sess, start_time):
    html = session.post(URL_GESTIONE_SQUADRE, data = {'id_gara': id_nuova_gara, 'NEW_SQ': 1, 'squadra_new': name}).text
    id_squadra = re.search(NEW_TEAM_REGEX % re.escape(name), html).group(1)

    for event in actions:
        match event:
            case (time, id_task):
                time = start_time + timedelta(minutes = time, seconds = 30)
                scheduler.enterabs(time.timestamp(), 0, set_jolly, argument = (id_nuova_gara, id_nuovo_sess, id_squadra, id_task))
            case (time, id_task, answer):
                time = start_time + timedelta(minutes = time, seconds = 30)
                scheduler.enterabs(time.timestamp(), 0, submit, argument = (id_nuova_gara, id_nuovo_sess, id_squadra, id_task, answer))

def run_contest(id_gara, id_sess, id_nuova_gara, id_nuovo_sess, start_time):
    # Decode contest information
    pdf = session.post(URL_STATS, data = {'id_gara': id_gara, 'id_sess': id_sess}).content

    # decoder = subprocess.run(['pdftotext', '-layout', '-nopgbrk', '-', '-'], input = pdf, stdout = subprocess.PIPE)
    # assert decoder.returncode == 0

    # question_id = [None]

    # for i, answer in enumerate(re.finditer(ANSWER_REGEX, decoder.stdout.decode('ascii', errors = 'ignore'))):
    #     html = session.post(URL_INSERIMENTO_DOMANDE, data = {
    #         'domanda_new': int(answer.group(1)),
    #         'NEW_DOM': 1,
    #         'numero': i + 1,
    #         'id_gara': id_nuova_gara
    #     }).text

    #     *_, last = re.finditer(QUESTION_REGEX, html)
    #     question_id.append(int(last.group(1)))

    html = session.post(URL_GARE, data = {'id_gara': id_gara, 'id_sess': id_sess}).text
    print(html)
    return

    # Decode team information
    for team in re.finditer(TEAM_REGEX, html):
        id_squadra, nome_squadra = team.group(1), filter_non_printable_chars(team.group(2))
        pdf = session.post(URL_STATS, data = {'id_gara': id_gara, 'id_sess': id_sess, 'id_squadra': id_squadra}).content

        decoder = subprocess.run(['pdftotext', '-layout', '-nopgbrk', '-', '-'], input = pdf, stdout = subprocess.PIPE)
        assert decoder.returncode == 0

        actions = []
        curr_problem = 0

        for m in re.finditer(STATS_REGEX, decoder.stdout.decode('ascii', errors = 'ignore')):
            match m.groups():
                case ('DOMANDA', _, _, _):
                    curr_problem += 1
                case (_, '(jolly)', _, _):
                    pass
                    # actions.append((9, curr_problem))
                case (_, _, time, answer):
                    actions.append((int(time), question_id[curr_problem], int(answer)))

        schedule_team_submissions(f"(fantasma) {nome_squadra}", actions, id_nuova_gara, id_nuovo_sess, start_time)

def main():
    global session, scheduler

    id_vecchia_gara = input("Inserire ID della gara da simulare: ")
    id_vecchia_sess = input("Inserire SESS della gara da simulare: ")
    h = int(input("Inserire ore: "))
    m = int(input("Inserire minuti: "))
    s = int(input("Inserire secondi: "))
    id_nuova_gara = input("Inserire ID della nuova gara: ")
    id_nuova_sess = input("Inserire SESS della nuova gara: ")

    session = requests.session()
    session.headers.update({'Cookie': COOKIE})

    start = datetime(datetime.now().year, datetime.now().month, datetime.now().day, h, m, s, 0)
    scheduler = sched.scheduler(time.time, time.sleep)

    run_contest(id_vecchia_gara, id_vecchia_sess, id_nuova_gara, id_nuova_sess, start)
    scheduler.run()

if __name__ == '__main__':
    main()
