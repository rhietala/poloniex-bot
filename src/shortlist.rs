extern crate diesel;

use super::diesel::prelude::*;
use super::models::Shortlist;
use super::schema::shortlist::dsl::*;

/// Returns the shortlist entry with the highest confidence score.
pub fn get_shortlist(
    connection: &mut PgConnection,
) -> Result<Option<Shortlist>, Box<dyn std::error::Error>> {
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
