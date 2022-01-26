extern crate diesel;

use diesel::prelude::*;
use diesel::{delete, sql_query};
use crate::schema::shortlist;

pub fn analyze(connection: &PgConnection, base: String, period: i32) -> Result<usize, diesel::result::Error> {
    println!("analyzing");

    delete(shortlist::table).execute(connection)?;
    sql_query(
        format!("
      WITH filtered_symbols AS (
        SELECT
          quote
        FROM
          candles
        WHERE
          base = '{base}'
          AND period = {period}
          AND timestamp > (current_timestamp - interval '24 hours')
        GROUP BY
          quote,
          base,
          period
        HAVING
          SUM(volume) > 3000
          AND MAX(timestamp) > (current_timestamp - interval '30 minutes')
      ),
      analyzed AS (
        SELECT
          DISTINCT ON (quote) quote,
          timestamp,
          average,
          -- average over previous 10 rows
          AVG(average) OVER(
            PARTITION BY quote,
            base,
            period
            ORDER BY
              timestamp ROWS BETWEEN 10 PRECEDING
              AND CURRENT ROW
          ) AS ma10,
          -- average over previous 30 rows
          AVG(average) OVER(
            PARTITION BY quote,
            base,
            period
            ORDER BY
              timestamp ROWS BETWEEN 30 PRECEDING
              AND CURRENT ROW
          ) AS ma30,
          -- average over previous 200 rows
          AVG(average) OVER(
            PARTITION BY quote,
            base,
            period
            ORDER BY
              timestamp ROWS BETWEEN 200 PRECEDING
              AND CURRENT ROW
          ) AS ma200
        FROM
          candles
        WHERE
          base = '{base}'
          AND period = {period}
          AND quote IN (SELECT quote FROM filtered_symbols)
        ORDER BY
          quote,
          timestamp DESC
      )
      INSERT INTO shortlist(quote, timestamp, average, confidence) (SELECT
        quote,
        NOW(),
        average,
        average / ma30 as confidence
      FROM
        (SELECT * FROM analyzed) AS analyzed
      WHERE
        -- actual logic: current value must be above 10-period moving average,
        -- which must be above 30-period MA, which must be above 200-period MA
        (
          average > ma10
          AND ma10 > ma30
          AND ma30 > ma200
        ) is true);
    ", base = base, period = period),
    )
    .execute(connection)
}