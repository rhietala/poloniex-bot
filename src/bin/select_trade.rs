extern crate diesel;
extern crate poloniex_bot;

use std::process::Command;
use std::thread;
use std::time::Duration;

use self::poloniex_bot::*;
use self::shortlist::*;
use self::trade::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let connection = &mut establish_connection();

    loop {
        println!("Looking up the best entry from shortlist");

        let shortlist = get_shortlist(connection).unwrap();

        match shortlist {
            Some(shortlist) => {
                println!("Found {:?}", shortlist.quote);

                if is_trade_open(connection, &shortlist).unwrap() {
                    println!("Trade already ongoing");
                } else {
                    println!("Starting to trade");
                    let trade = create_trade(connection, &shortlist).unwrap();
                    Command::new("./target/release/do_trade")
                        .arg(trade.id.to_string())
                        .spawn()
                        .expect("Failed to fork process for trade");
                }
            }
            None => (),
        };
        thread::sleep(Duration::from_secs(60));
    }
}
