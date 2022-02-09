extern crate diesel;

use crate::schema::shortlist;
use diesel::prelude::*;
use diesel::{delete, sql_query};

const MA_SHORT: i32 = 10;
const MA_MED: i32 = 30;
const MA_LONG: i32 = 200;

pub fn analyze(
    connection: &PgConnection,
    base: String,
    period: i32,
) -> Result<usize, diesel::result::Error> {
    println!("analyzing");

    let max_seconds = period * MA_LONG;

    delete(shortlist::table).execute(connection)?;
    sql_query(format!(
        "
      WITH filtered_symbols AS (
        SELECT
          quote
        FROM
          candles
        WHERE
          base = '{base}'
          AND period = {period}
          AND timestamp > (current_timestamp - interval '{max_seconds} seconds')
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
          -- filter out those that are missing more than 5 candles from the longest MA period
          AND count(*) > ({max_seconds} / {period}) - 5
      ),
      analyzed AS (
        SELECT
          DISTINCT ON (quote) quote,
          timestamp,
          average,
          -- short moving average
          AVG(average) OVER(
            PARTITION BY quote,
            base,
            period
            ORDER BY
              timestamp ROWS BETWEEN {ma_short} PRECEDING
              AND CURRENT ROW
          ) AS ma_short,
          -- medium moving average
          AVG(average) OVER(
            PARTITION BY quote,
            base,
            period
            ORDER BY
              timestamp ROWS BETWEEN {ma_med} PRECEDING
              AND CURRENT ROW
          ) AS ma_med,
          -- long moving average
          AVG(average) OVER(
            PARTITION BY quote,
            base,
            period
            ORDER BY
              timestamp ROWS BETWEEN {ma_long} PRECEDING
              AND CURRENT ROW
          ) AS ma_long
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
        average / ma_med as confidence
      FROM
        (SELECT * FROM analyzed) AS analyzed
      WHERE
        (
          -- actual logic: current value must be above 10-period moving average,
          -- which must be above 30-period MA, which must be above 200-period MA
          average > ma_short
          AND ma_short > ma_med
          AND ma_med > ma_long
          -- filter out too small changes from moving average (stablecoins)
          AND (average / ma_med) > 1.001
          -- and too large changes, too strange situations
          AND (average / ma_med) < 1.100
        ) is true);
    ",
        base = base,
        period = period,
        max_seconds = max_seconds,
        ma_short = MA_SHORT,
        ma_med = MA_MED,
        ma_long = MA_LONG
    ))
    .execute(connection)
}
