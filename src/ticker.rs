extern crate diesel;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

const API_URL: &str = "https://poloniex.com/public";

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub struct PoloniexTicker {
    pub id: i32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Ticker {
    pub base: String,
    pub quote: String,
}

pub fn return_ticker(base: String) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let client = reqwest::blocking::Client::new();

    println!("Fetching tickers");

    let ret: HashMap<String, PoloniexTicker> = client
        .get(API_URL)
        .query(&[("command", "returnTicker")])
        .send()?
        .json::<HashMap<String, PoloniexTicker>>()?;

    let mut tickers: Vec<Ticker> = vec![];

    for key in ret.into_keys() {
        let mut splitted = key.split("_");
        let base = splitted.next().unwrap().to_string();
        let quote = splitted.next().unwrap().to_string();

        tickers.push(Ticker {
            base: base,
            quote: quote,
        })
    }

    let quotes: Vec<String> = tickers
        .into_iter()
        .filter(|t| t.base == base)
        .map(|t| t.quote)
        .collect();

    Ok(quotes)
}
