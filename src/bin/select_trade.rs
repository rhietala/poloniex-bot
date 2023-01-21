extern crate diesel;
extern crate poloniex_bot;

use std::collections::HashMap;
use std::process::{Child, Command};
use std::thread;
use std::time::Duration;

use self::poloniex_bot::*;
use self::shortlist::*;
use self::trade::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let connection = &mut establish_connection();

    let mut processes: HashMap<i32, Child> = HashMap::new();

    loop {
        println!("Getting shortlist");

        // kill all trade processes
        for (_, mut child) in processes.into_iter() {
            child.kill()?;
        }

        processes = HashMap::new();

        // restart all open trades
        for trade in get_trades(connection)? {
            println!("Reopening trade {:?}", trade.quote);
            let process = Command::new("./target/release/do_trade")
                .arg(trade.id.to_string())
                .spawn()
                .expect("Failed to fork process for trade");
            processes.insert(trade.id, process);
        }

        // start new trades from shortlist

        for s in get_shortlist(connection).unwrap() {
            if !is_trade_open(connection, &s).unwrap() {
                println!("Starting to trade {:?}", s.quote);
                let trade = create_trade(connection, &s).unwrap();
                let process = Command::new("./target/release/do_trade")
                    .arg(trade.id.to_string())
                    .spawn()
                    .expect("Failed to fork process for trade");
                processes.insert(trade.id, process);
            }
        }
        thread::sleep(Duration::from_secs(120));
    }
}
