#![allow(clippy::module_inception)]

#[macro_use]
extern crate rocket;

use std::env;

use rocket::fs::{relative, FileServer};
use rocket::http::Status;
use rocket::State;
use rocket_db_pools::diesel;
use rocket_db_pools::{Connection, Database};
use rocket_dyn_templates::Template;

mod contest;
mod error;
mod models;
mod schema;

#[derive(Database)]
#[database("gas_simulator")]
pub struct DB(diesel::PgPool);

struct PhiQuadroLogin {
    username: String,
    password: String,
}

#[get("/")]
async fn index(db: Connection<DB>, phi: &State<PhiQuadroLogin>) -> Result<&'static str, Status> {
    let contest_creation = contest::import::create_contest(
        db,
        phi.inner(),
        "Suscontest",
        12000,
        1,
        4800,
        chrono::offset::Utc::now().naive_utc(),
        5,
    )
    .await?;

    Ok("Hello world")
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
        .mount("/", routes![index])
}
