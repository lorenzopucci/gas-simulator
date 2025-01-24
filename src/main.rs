#![allow(clippy::module_inception)]

#[macro_use]
extern crate rocket;

use rocket::fs::{relative, FileServer};
use rocket_db_pools::Database;
use rocket_dyn_templates::Template;

mod contest;
mod db;

#[launch]
fn rocket() -> _ {
    rocket::build()
        .attach(db::DB::init())
        .attach(Template::fairing())
        .mount(
            "/static",
            FileServer::new(relative!("/static"), rocket::fs::Options::None),
        )
}
