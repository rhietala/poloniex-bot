extern crate diesel;

use super::diesel::prelude::*;
use super::models::*;
use super::BASE;
use chrono::Utc;

/// Creates a trade based on shortlist entry
pub fn create_trade(
    connection: &mut PgConnection,
    shortlist: &Shortlist,
) -> Result<Trade, Box<dyn std::error::Error>> {
    use super::schema::trades;

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

/// Checks whether a trade is open given a shortlist entry
pub fn is_trade_open(
    connection: &mut PgConnection,
    shortlist: &Shortlist,
) -> Result<bool, Box<dyn std::error::Error>> {
    use super::schema::trades::dsl::*;

    let rows = trades
        .filter(base.eq(BASE))
        .filter(quote.eq(shortlist.quote.clone()))
        .filter(close_at.is_null())
        .limit(1)
        .load::<Trade>(connection)
        .unwrap();

    Ok(rows.len() > 0)
}
