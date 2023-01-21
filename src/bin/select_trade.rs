extern crate diesel;
extern crate poloniex_bot;

use std::process::Command;
use std::thread;
use std::time::Duration;

use self::models::*;
use self::poloniex_bot::*;
use self::shortlist::*;
use self::trade::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let connection = &mut establish_connection();

    loop {
        println!("Getting shortlist");

        let shortlist: Vec<Shortlist> = get_shortlist(connection).unwrap();

        for s in shortlist {
            println!("Found {:?}", s.quote);

            if is_trade_open(connection, &s).unwrap() {
                println!("Trade already ongoing");
            } else {
                println!("Starting to trade");
                let trade = create_trade(connection, &s).unwrap();
                Command::new("./target/release/do_trade")
                    .arg(trade.id.to_string())
                    .spawn()
                    .expect("Failed to fork process for trade");
            }
        }
        thread::sleep(Duration::from_secs(60));
    }
}
