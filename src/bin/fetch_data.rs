extern crate diesel;
extern crate poloniex_bot;

use self::diesel::prelude::*;
use self::models::*;
use self::poloniex_bot::*;

// cargo run --bin fetch_data

const PERIOD: i32 = 900;
const CANDLES: i32 = 400;
const BASE: &str = "USDT";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    use self::chart_data::*;
    use self::schema::candles;
    use self::shortlist_logic::{update_shortlist, update_trades};
    use self::ticker::*;

    let quotes = return_ticker(BASE.to_string()).unwrap();

    let connection = &mut establish_connection();
    let period = PERIOD;

    for quote in quotes {
        match return_chart_data(connection, BASE.to_string(), quote.clone(), PERIOD, CANDLES) {
            Ok(chart_datas) => {
                let candles: Vec<Candle> = chart_datas
                    .iter()
                    .map(|&cd| {
                        chart_data_to_candle(BASE.to_string(), quote.to_string(), period, cd)
                    })
                    .collect();

                println!("{}: {}", quote, candles.len());
                diesel::insert_into(candles::table)
                    .values(&candles)
                    .execute(connection)?;
            }
            Err(e) => println!("Error fetching {}: {}", quote, e),
        }
    }

    update_trades(connection, BASE.to_string(), period)?;
    update_shortlist(connection, BASE.to_string(), period)?;

    Ok(())
}
