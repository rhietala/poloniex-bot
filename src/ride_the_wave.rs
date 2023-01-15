extern crate diesel;

use crate::schema::shortlist;
use diesel::prelude::*;
use diesel::{delete, sql_query};

const MA_SHORT: i32 = 10;
const MA_MED: i32 = 30;
const MA_LONG: i32 = 200;

pub fn get_analyze_sql(base: String, period: i32) -> String {
    format!(
        "analyzed AS (
        SELECT
          DISTINCT ON (quote) quote,
          timestamp,
          average,
          volume,
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
          ) AS ma_long,
          -- short window volume
          SUM(volume * average) OVER(
            PARTITION BY quote,
            base,
            period
            ORDER BY
              timestamp ROWS BETWEEN {ma_short} PRECEDING
              AND CURRENT ROW
          ) AS base_volume_short,
          MAX((high - low) / low) OVER(
            PARTITION BY quote,
            base,
            period
            ORDER BY
              timestamp ROWS BETWEEN {ma_med} PRECEDING
              AND CURRENT ROW
          ) AS volatility_med
        FROM
          candles
        WHERE
          base = '{base}'
          AND period = {period}
          AND quote IN (SELECT quote FROM filtered_symbols)
        ORDER BY
          quote,
          timestamp DESC
      )",
        base = base,
        period = period,
        ma_short = MA_SHORT,
        ma_med = MA_MED,
        ma_long = MA_LONG
    )
}

pub fn update_shortlist(
    connection: &mut PgConnection,
    base: String,
    period: i32,
) -> Result<usize, diesel::result::Error> {
    println!("updating shortlist");

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
          -- filter out stablecoins
          AND quote NOT IN ('TUSD', 'USDC', 'USDT', 'PAX', 'GUSD', 'DAI', 'BUSD')
        GROUP BY
          quote,
          base,
          period
        HAVING
          -- no recent data
          MAX(timestamp) > (current_timestamp - interval '30 minutes')
          -- too small minimum quote value (these have too high %-change with single pips)
          AND MIN(average) > 1e-6
          -- more than 5 candles missing from the longest MA period
          AND count(*) > ({max_seconds} / {period}) - 5
      ),
      {analyzed}
      INSERT INTO shortlist(quote, timestamp, average, target, confidence) (SELECT
        quote,
        NOW(),
        average,
        ma_short as target,
        average / ma_med as confidence
      FROM
        (SELECT * FROM analyzed) AS analyzed
      WHERE
        (
          -- filter out those with too small volume in base unit (USDT), short window
          base_volume_short > 2000
          -- actual logic: current value must be above 10-period moving average,
          -- which must be above 30-period MA, which must be above 200-period MA
          AND average > ma_short
          AND ma_short > ma_med
          AND ma_med > ma_long
          -- too large change, too strange situation
          AND (average / ma_med) < 1.100
          -- too big %-change in last candle
          AND volatility_med < 0.05
        ) is true);
    ",
        max_seconds = max_seconds,
        base = base.clone(),
        period = period,
        analyzed = get_analyze_sql(base, period)
    ))
    .execute(connection)
}

pub fn update_trades(
    connection: &mut PgConnection,
    base: String,
    period: i32,
) -> Result<usize, diesel::result::Error> {
    println!("updating trades");

    sql_query(format!(
        "
      WITH filtered_symbols AS (
        SELECT
          quote
        FROM
          trades
      ),
      {analyzed}
      UPDATE
        trades
      SET
        target = temp.target
      FROM
        (
          SELECT
            quote,
            ma_short
          FROM
            analyzed
        ) as temp(quote, target)
      WHERE
        trades.quote = temp.quote AND
        trades.close_at IS NULL
    ",
        analyzed = get_analyze_sql(base, period)
    ))
    .execute(connection)
}
