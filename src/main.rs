#![allow(clippy::module_inception)]
#![allow(clippy::too_many_arguments)]

#[macro_use]
extern crate rocket;

use std::env;

use contest::contest::Contest;
use contest::fetch::fetch_contest;
use error::IntoStatusResult;
use rocket::fs::{relative, FileServer};
use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::State;
use rocket_db_pools::diesel;
use rocket_db_pools::{Connection, Database};
use rocket_dyn_templates::Template;

mod contest;
mod error;
mod model;
mod schema;

#[derive(Database)]
#[database("gas_simulator")]
pub struct DB(diesel::PgPool);

struct PhiQuadroLogin {
    username: String,
    password: String,
}

#[get("/create")]
async fn index(mut db: Connection<DB>, phi: &State<PhiQuadroLogin>) -> Result<&'static str, Status> {
    let contest_creation = contest::import::create_contest(
        &mut db,
        phi.inner(),
        "Suscontest",
        16463,
        1,
        7200,
        chrono::offset::Utc::now().naive_utc(),
        3,
        6000,
    )
    .await?;

    Ok("Hello world")
}

#[get("/contest/<id>")]
async fn show_contest(id: i32, mut db: Connection<DB>) -> Result<Template, Status> {
    match fetch_contest(&mut db, id).await.attach_status(Status::InternalServerError)? {
        Some(contest) => Ok(Template::render("ranking", contest)),
        None => Err(Status::NotFound),
    }
}

#[launch]
fn rocket() -> _ {
    tracing_subscriber::fmt().init();
    dotenvy::dotenv().expect("could not open .env file");

    rocket::build()
        .attach(DB::init())
        .attach(Template::fairing())
        .manage(PhiQuadroLogin {
            username: env::var("USERNAME").expect("please set a username in .env"),
            password: env::var("PASSWORD").expect("please set a password in .env"),
        })
        .mount(
            "/static",
            FileServer::new(relative!("/static"), rocket::fs::Options::None),
        )
        .mount("/", routes![index, show_contest])
}
