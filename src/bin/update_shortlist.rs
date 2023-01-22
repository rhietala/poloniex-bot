extern crate diesel;
extern crate poloniex_bot;

use self::poloniex_bot::*;
use self::shortlist_logic::{update_shortlist, update_trades};

// cargo run --bin update_shortlist

const PERIOD: i32 = 900;
const BASE: &str = "USDT";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let connection = &mut establish_connection();
    let period = PERIOD;

    update_trades(connection, BASE.to_string(), period)?;
    update_shortlist(connection, BASE.to_string(), period)?;

    Ok(())
}
