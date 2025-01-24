use rocket_db_pools::sqlx::{self, Postgres};
use rocket_db_pools::Database;

#[derive(Database)]
#[database("gas_simulator")]
pub struct DB(sqlx::Pool<Postgres>);
