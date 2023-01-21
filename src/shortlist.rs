extern crate diesel;

use super::diesel::prelude::*;
use super::models::Shortlist;
use super::schema::shortlist::dsl::*;

/// Returns the shortlist entry with the highest confidence score.
pub fn get_shortlist(
    connection: &mut PgConnection,
) -> Result<Vec<Shortlist>, Box<dyn std::error::Error>> {
    let rows = shortlist
        .order(confidence.desc())
        .load::<Shortlist>(connection)
        .unwrap();

    let quotes: Vec<String> = rows.clone().iter().map(|row| row.quote.clone()).collect();

    diesel::delete(shortlist.filter(quote.eq_any(quotes)))
        .execute(connection)
        .unwrap();

    Ok(rows)
}
