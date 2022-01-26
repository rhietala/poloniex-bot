extern crate diesel;

use chrono::{TimeZone, Utc};
use serde::{Deserialize, Serialize};

use crate::models::*;
use diesel::prelude::*;

const API_URL: &str = "https://poloniex.com/public";

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub struct PoloniexChartData {
    pub date: i32,
    pub high: f32,
    pub low: f32,
    pub open: f32,
    pub close: f32,
    pub volume: f32,
    #[serde(rename = "quoteVolume")]
    pub quote_volume: f32,
    #[serde(rename = "weightedAverage")]
    pub weighted_average: f32,
}

pub fn chart_data_to_candle(base: String, quote: String, period: i32, cd: PoloniexChartData) -> Candle {
    Candle {
        base: base,
        quote: quote,
        period: period,
        timestamp: Utc.timestamp(cd.date.into(), 0),
        high: Some(cd.high),
        low: Some(cd.low),
        open: Some(cd.open),
        close: Some(cd.close),
        average: Some(cd.weighted_average),
        volume: Some(cd.volume),
    }
}

pub fn fetch_data(
    connection: &PgConnection,
    base: String,
    quote: String,
    period: i32,
    max_candles: i32
) -> Result<Vec<PoloniexChartData>, Box<dyn std::error::Error>> {
    let client = reqwest::blocking::Client::new();
    let now = Utc::now().timestamp();
    let end = i32::try_from(now)?;
    let start = get_start_timestamp(connection, base.clone(), quote.clone(), period, max_candles)?;

    println!(
        "{}: {}?command=returnChartData&currencyPair={}&period={}&start={}&end={}",
        quote,
        API_URL,
        format!("{}_{}", base, quote).as_str(),
        period.to_string().as_str(),
        start.to_string().as_str(),
        end.to_string().as_str()
    );

    let ret: Vec<PoloniexChartData> = client
        .get(API_URL)
        .query(&[
            ("command", "returnChartData"),
            ("currencyPair", format!("{}_{}", base, quote).as_str()),
            ("period", period.to_string().as_str()),
            ("start", start.to_string().as_str()),
            ("end", end.to_string().as_str()),
        ])
        .send()?
        .json::<Vec<PoloniexChartData>>()?
        .into_iter()
        .filter(|cd| cd.date != 0)
        .collect();

    Ok(ret)
}

pub fn get_start_timestamp(
    connection: &PgConnection,
    base_p: String,
    quote_p: String,
    period_p: i32,
    max_candles: i32,
) -> Result<i32, Box<dyn std::error::Error>> {
    use crate::schema::candles::dsl::*;

    let results = candles
        .filter(base.eq(base_p))
        .filter(quote.eq(quote_p))
        .filter(period.eq(period))
        .order(timestamp.desc())
        .limit(1)
        .load::<Candle>(connection)
        .expect("Error loading candles");

    // candle found
    for candle in results {
        let last_timestamp: i32 = candle.timestamp.timestamp().try_into().unwrap();
        return Ok(last_timestamp + period_p);
    }

    let now = Utc::now().timestamp();
    let end = i32::try_from(now)?;
    let start = end - (max_candles * period_p);

    Ok(start)
}
