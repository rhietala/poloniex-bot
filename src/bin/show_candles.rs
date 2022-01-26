extern crate poloniex_bot;
extern crate diesel;

use self::poloniex_bot::*;
use self::models::*;
use self::diesel::prelude::*;

fn main() {
    use poloniex_bot::schema::candles::dsl::*;

    let connection = establish_connection();
    let results = candles
        .filter(base.eq("USDT"))
        .filter(quote.eq("LRC"))
        .filter(period.eq(900))
        .order(timestamp.desc())
        .limit(1)
        .load::<Candle>(&connection)
        .expect("Error loading candles");

    println!("Displaying {} candles", results.len());
    for candle in results {
        match candle.average {
            Some(x) => { println!("{} {}", candle.timestamp.timestamp(), x) }
            None => {}
        }
    }
}
