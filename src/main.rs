#![allow(clippy::module_inception)]
#![allow(clippy::too_many_arguments)]

#[macro_use]
extern crate rocket;

use std::env;

use rocket::fs::{relative, FileServer};
use rocket_db_pools::diesel;
use rocket_db_pools::Database;
use rocket_dyn_templates::Template;

mod api;
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
        .mount("/", FileServer::new(relative!("/static"), rocket::fs::Options::None))
        .mount("/", contest::pages::routes())
        .mount("/", api::routes())
}
