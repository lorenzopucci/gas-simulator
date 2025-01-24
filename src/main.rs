#![allow(clippy::module_inception)]

#[macro_use]
extern crate rocket;

use rocket::fs::{relative, FileServer};
use rocket_db_pools::diesel;
use rocket_db_pools::{Connection, Database};
use rocket_dyn_templates::Template;

mod contest;
mod models;
mod schema;

#[derive(Database)]
#[database("gas_simulator")]
pub struct DB(diesel::PgPool);

#[get("/")]
async fn index(db: Connection<DB>) -> &'static str {
    //contest::import::import_contest(15712, 1).await.unwrap();

    contest::import::create_contest(
        db,
        "Suscontest",
        12000,
        1,
        4800,
        chrono::offset::Utc::now().naive_utc(),
        5,
    )
    .await
    .unwrap();

    "Hello, world!"
}

#[launch]
fn rocket() -> _ {
    rocket::build()
        .attach(DB::init())
        .attach(Template::fairing())
        .mount(
            "/static",
            FileServer::new(relative!("/static"), rocket::fs::Options::None),
        )
        .mount("/", routes![index])
}
