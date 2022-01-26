use chrono::{DateTime, Utc};

use super::schema::candles;

#[derive(Debug, Insertable, Queryable)]
#[table_name="candles"]
pub struct Candle {
    pub base: String,
    pub quote: String,
    pub period: i32,
    pub timestamp: DateTime<Utc>,
    pub high: Option<f32>,
    pub low: Option<f32>,
    pub open: Option<f32>,
    pub close: Option<f32>,
    pub average: Option<f32>,
    pub volume: Option<f32>,
}