use chrono::{DateTime, Utc};

use super::schema::{candles, shortlist, trade};

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

#[derive(Debug, Insertable, Queryable)]
#[table_name="shortlist"]
pub struct Shortlist {
    pub quote: String,
    pub timestamp: DateTime<Utc>,
    pub average: f32,
    pub confidence: f32,
}

#[derive(Debug, Insertable, Queryable)]
#[table_name="trade"]
pub struct Trade {
    pub id: i32,
    pub base: String,
    pub quote: String,
    pub open_at: DateTime<Utc>,
    pub close_at: Option<DateTime<Utc>>,
    pub target: f32,
    pub open: Option<f32>,
    pub close: Option<f32>,
}
