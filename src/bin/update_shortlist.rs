extern crate diesel;
extern crate poloniex_bot;

use self::poloniex_bot::*;

// cargo run --bin update_shortlist

const PERIOD: i32 = 900;
const BASE: &str = "USDT";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    use self::shortlist_logic::update_shortlist;

    let connection = &mut establish_connection();
    let period = i32::try_from(PERIOD)?;

    update_shortlist(connection, BASE.to_string(), period)?;

    Ok(())
}
