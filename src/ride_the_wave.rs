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
          -- filter out instruments
          AND quote NOT LIKE '%BULL'
          AND quote NOT LIKE '%BEAR'
        GROUP BY
          quote,
          base,
          period
        HAVING
          -- filter out those with too small daily volume in base unit (USDT)
          SUM(volume) > 3000
          -- filter out those that don't have recent data
          AND MAX(timestamp) > (current_timestamp - interval '30 minutes')
          -- filter out those with too small minimum quote value
          -- (these have too high %-change with single pips)
          AND MIN(average) > 1e-6
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
        (
          -- actual logic: current value must be above 10-period moving average,
          -- which must be above 30-period MA, which must be above 200-period MA
          average > ma10
          AND ma10 > ma30
          AND ma30 > ma200
          -- filter out too small changes from moving average (stablecoins)
          AND average / ma30 > 0.001
          -- and too large changes, too strange situations
          AND average / ma30 < 0.100
        ) is true);
    ", base = base, period = period),
    )
    .execute(connection)
}