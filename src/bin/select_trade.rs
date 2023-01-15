extern crate diesel;
extern crate poloniex_bot;

use chrono::Utc;

use self::diesel::prelude::*;
use self::models::*;
use self::order_book::do_trade;
use self::poloniex_bot::*;

const BASE: &str = "USDT";

fn create_trade(
    connection: &mut PgConnection,
    shortlist: &Shortlist,
) -> Result<Trade, Box<dyn std::error::Error>> {
    use self::schema::trades;

    let new_trade = NewTrade {
        base: BASE.to_string(),
        quote: shortlist.quote.clone(),
        target: shortlist.target,
        open_average: shortlist.average,
        open_at: Utc::now(),
    };

    let trade = diesel::insert_into(trades::table)
        .values(&new_trade)
        .get_result::<Trade>(connection)
        .unwrap();

    Ok(trade)
}

fn is_trade_open(
    connection: &mut PgConnection,
    shortlist: &Shortlist,
) -> Result<bool, Box<dyn std::error::Error>> {
    use self::schema::trades::dsl::*;

    let rows = trades
        .filter(base.eq(BASE))
        .filter(quote.eq(shortlist.quote.clone()))
        .filter(close_at.is_null())
        .limit(1)
        .load::<Trade>(connection)
        .unwrap();

    if rows.len() > 0 {
        Ok(true)
    } else {
        Ok(false)
    }
}

fn get_shortlist(
    connection: &mut PgConnection,
) -> Result<Option<Shortlist>, Box<dyn std::error::Error>> {
    use self::schema::shortlist::dsl::*;

    let rows = shortlist
        .limit(1)
        .order(confidence.desc())
        .load::<Shortlist>(connection)
        .unwrap();

    match rows.get(0) {
        Some(row) => {
            diesel::delete(shortlist.filter(quote.eq(row.quote.clone())))
                .execute(connection)
                .unwrap();
            Ok(Some((*row).clone()))
        }
        None => Ok(None),
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let connection = &mut establish_connection();

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
                do_trade(connection, &trade);
            }
        }
        None => (),
    };

    Ok(())
}
