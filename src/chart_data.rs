extern crate diesel;

use chrono::{TimeZone, Utc};
use serde::{Deserialize, Deserializer, Serialize};

use crate::models::*;
use diesel::prelude::*;

const API_URL: &str = "https://poloniex.com/public";

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub struct PoloniexChartData {
    #[serde(deserialize_with = "deserialize_date")]
    pub date: i64,
    #[serde(deserialize_with = "deserialize_f32_from_str")]
    pub high: f32,
    #[serde(deserialize_with = "deserialize_f32_from_str")]
    pub low: f32,
    #[serde(deserialize_with = "deserialize_f32_from_str")]
    pub open: f32,
    #[serde(deserialize_with = "deserialize_f32_from_str")]
    pub close: f32,
    #[serde(deserialize_with = "deserialize_f32_from_str")]
    pub volume: f32,
    #[serde(rename = "quoteVolume", deserialize_with = "deserialize_f32_from_str")]
    pub quote_volume: f32,
    #[serde(
        rename = "weightedAverage",
        deserialize_with = "deserialize_f32_from_str"
    )]
    pub weighted_average: f32,
}

fn deserialize_date<'de, D>(deserializer: D) -> Result<i64, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    s[..s.len() - 3]
        .parse::<i64>()
        .map_err(serde::de::Error::custom)
}

fn deserialize_f32_from_str<'de, D>(deserializer: D) -> Result<f32, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    s.parse::<f32>().map_err(serde::de::Error::custom)
}

pub fn chart_data_to_candle(
    base: String,
    quote: String,
    period: i32,
    cd: PoloniexChartData,
) -> Candle {
    Candle {
        base: base,
        quote: quote,
        period: period,
        timestamp: Utc.timestamp_opt(cd.date, 0).unwrap(),
        high: Some(cd.high),
        low: Some(cd.low),
        open: Some(cd.open),
        close: Some(cd.close),
        average: Some(cd.weighted_average),
        volume: Some(cd.volume),
    }
}

pub fn return_chart_data(
    connection: &mut PgConnection,
    base: String,
    quote: String,
    period: i32,
    max_candles: i32,
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

    let response = client
        .get(API_URL)
        .query(&[
            ("command", "returnChartData"),
            ("currencyPair", format!("{}_{}", base, quote).as_str()),
            ("period", period.to_string().as_str()),
            ("start", start.to_string().as_str()),
            ("end", end.to_string().as_str()),
        ])
        .send()?;

    if response.status().is_success() {
        let chart_data: Vec<PoloniexChartData> = response
            .json::<Vec<PoloniexChartData>>()?
            .into_iter()
            .filter(|cd| cd.date != 0)
            .collect();
        Ok(chart_data)
    } else {
        Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Request not successful: {}", response.status()),
        )))
    }
}

pub fn get_start_timestamp(
    connection: &mut PgConnection,
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
