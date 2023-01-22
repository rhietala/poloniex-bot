extern crate diesel;
extern crate poloniex_bot;

use std::process::{Child, Command};
use std::thread;
use std::time::Duration;

use self::models::Trade;
use self::poloniex_bot::*;
use self::shortlist::*;
use self::trade::*;
use self::trade_logic::log_trade;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let connection = &mut establish_connection();

    let mut processes: Vec<(Trade, Child)> = vec![];

    loop {
        println!("Process manager loop start");

        // kill all trade processes

        for (trade, mut process) in processes.into_iter() {
            log_trade(&trade, format!("killing process {}", process.id()));
            process.kill()?;
        }

        processes = vec![];

        // restart all open trades

        for trade in get_trades(connection)? {
            let process = Command::new("./target/release/do_trade")
                .arg(trade.id.to_string())
                .spawn()
                .expect("Failed to fork process for trade");
            log_trade(&trade, format!("reopening process {}", process.id()));
            processes.push((trade, process));
        }

        // start new trades from shortlist

        for s in get_shortlist(connection).unwrap() {
            if !is_trade_open(connection, &s).unwrap() {
                let trade = create_trade(connection, &s).unwrap();
                let process = Command::new("./target/release/do_trade")
                    .arg(trade.id.to_string())
                    .spawn()
                    .expect("Failed to fork process for trade");
                log_trade(&trade, format!("starting process {}", process.id()));
                processes.push((trade, process));
            }
        }
        thread::sleep(Duration::from_secs(120));
    }
}
