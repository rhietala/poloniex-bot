#[macro_use]
extern crate diesel;
extern crate dotenv;

pub mod chart_data;
pub mod models;
pub mod order_book;
pub mod schema;
pub mod shortlist;
pub mod shortlist_logic;
pub mod ticker;
pub mod trade;
pub mod trade_logic;

use diesel::pg::PgConnection;
use diesel::prelude::*;
use dotenv::dotenv;
use std::env;

pub const BASE: &str = "USDT";

pub fn establish_connection() -> PgConnection {
    dotenv().ok();

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    PgConnection::establish(&database_url)
        .unwrap_or_else(|_| panic!("Error connecting to {}", database_url))
}
