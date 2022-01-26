extern crate diesel;
extern crate poloniex_bot;

use self::diesel::prelude::*;
use self::models::*;
use self::poloniex_bot::*;

// cargo run --bin fetch_data

const PERIOD: i32 = 900;
const CANDLES: i32 = 400;
const BASE: &str = "USDT";

fn read_quotes() -> Result<String, Box<dyn std::error::Error>> {
    use std::fs::File;
    use std::io::prelude::*;

    let mut file = File::open("included-symbols.txt")?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;

    Ok(contents)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    use self::poloniex::*;
    use self::ride_the_wave::{analyze};
    use self::schema::candles;

    let connection = establish_connection();
    let period = i32::try_from(PERIOD)?;
    let quotes = read_quotes()?;
    for quote in quotes.split('\n') {
        let chart_datas = fetch_data(
            &connection,
            BASE.to_string(),
            quote.to_string().clone(),
            PERIOD,
            CANDLES,
        )?;

        let candles: Vec<Candle> = chart_datas
            .iter()
            .map(|&cd| chart_data_to_candle(BASE.to_string(), quote.to_string(), period, cd))
            .collect();

        diesel::insert_into(candles::table)
            .values(&candles)
            .execute(&connection)?;
    }

    analyze(&connection, BASE.to_string(), period)?;

    Ok(())
}
