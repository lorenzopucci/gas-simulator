use rocket::http::Status;
use rocket::Route;
use rocket_dyn_templates::Template;
use rocket_dyn_templates::context;

use crate::api::ApiUser;


#[get("/")]
async fn get_privacy_policy(user: Option<ApiUser>) -> Template {
    Template::render("privacy_policy", context! { user })
}

pub fn routes() -> Vec<Route> {
    routes![get_privacy_policy,]
}
