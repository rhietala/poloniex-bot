extern crate diesel;
extern crate poloniex_bot;

use self::poloniex_bot::*;
use self::trade_logic::do_trade;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = std::env::args().collect::<Vec<_>>();
    let connection = &mut establish_connection();

    let trade_id: i32 = match args.get(1) {
        Some(id) => id.parse().unwrap(),
        None => {
            println!("Usage: {} <trade_id>", args[0]);
            return Ok(());
        }
    };

    do_trade(connection, trade_id.clone());

    println!("do_trade {} finished", trade_id);
    Ok(())
}
