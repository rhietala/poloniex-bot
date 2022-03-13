extern crate diesel;
extern crate poloniex_bot;

use chrono::Utc;

use self::diesel::prelude::*;
use self::models::*;
use self::poloniex_bot::*;

const BASE: &str = "USDT";

fn create_trade(
    connection: PgConnection,
    shortlist: &Shortlist,
) -> Result<Trade, Box<dyn std::error::Error>> {
    use self::schema::trades;

    let new_trade = NewTrade {
        base: BASE.to_string(),
        quote: shortlist.quote.clone(),
        target: shortlist.average,
        open_average: shortlist.average,
        open_at: Utc::now(),
    };

    let trade = diesel::insert_into(trades::table)
        .values(&new_trade)
        .get_result::<Trade>(&connection)
        .unwrap();

    Ok(trade)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let connection = establish_connection();

    let shortlist_rows = || {
        use self::schema::shortlist::dsl::*;

        return shortlist
            .limit(1)
            .order(confidence.desc())
            .load::<Shortlist>(&connection)
            .unwrap();
    };

    let rows = shortlist_rows();
    let row = rows.get(0);

    println!("{:?}", row);

    let trade: Option<Trade> = match row {
        Some(shortlist) => { Some(create_trade(connection, shortlist).unwrap()) }
        None => None
    };

    println!("{:?}", trade);

    Ok(())
}
