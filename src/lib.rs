#[macro_use]
extern crate diesel;
extern crate dotenv;

pub mod chart_data;
pub mod models;
pub mod order_book;
pub mod ride_the_wave;
pub mod schema;
pub mod ticker;

use diesel::pg::PgConnection;
use diesel::prelude::*;
use dotenv::dotenv;
use std::env;

pub fn establish_connection() -> PgConnection {
    dotenv().ok();

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    PgConnection::establish(&database_url)
        .unwrap_or_else(|_| panic!("Error connecting to {}", database_url))
}
