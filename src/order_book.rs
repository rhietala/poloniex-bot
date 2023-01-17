use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

pub const HEARTBEAT_ID: u32 = 1010;
pub const F64_EPSILON: f64 = 1e-10;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct PoloniexMessage {
    pub channel_id: u32,
    pub sequence_num: Option<u32>,
    pub messages: Vec<Value>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
struct PoloniexOrderBook {
    #[serde(rename = "currencyPair")]
    pub currency_pair: String,
    #[serde(rename = "orderBook")]
    pub order_book: (HashMap<String, String>, HashMap<String, String>),
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum OrderType {
    Bid,
    Ask,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct OrderBookEntry {
    order_type: OrderType,
    size: f64,
    pub price: f64,
}

pub type OrderBook = HashMap<String, OrderBookEntry>;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct OrderBookMiddle {
    pub highest_bid: Option<OrderBookEntry>,
    pub lowest_ask: Option<OrderBookEntry>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Command {
    pub command: String,
    pub channel: String,
}

pub fn parse_message(input: String) -> PoloniexMessage {
    let parsed: Vec<Value> = serde_json::from_str(&input).unwrap();
    let channel_id: u32 = serde_json::from_value(parsed[0].clone()).unwrap();
    match channel_id {
        HEARTBEAT_ID => PoloniexMessage {
            channel_id: channel_id,
            sequence_num: None,
            messages: vec![],
        },
        _ => {
            let sequence_num: u32 = serde_json::from_value(parsed[1].clone()).unwrap();
            let messages: Vec<Value> = serde_json::from_value(parsed[2].clone()).unwrap();
            PoloniexMessage {
                channel_id: channel_id,
                sequence_num: Some(sequence_num),
                messages: messages,
            }
        }
    }
}

pub fn find_middle(order_book: OrderBook) -> OrderBookMiddle {
    let mut highest_bid: Option<OrderBookEntry> = None;
    let mut lowest_ask: Option<OrderBookEntry> = None;

    for entry in order_book.into_values() {
        match entry.order_type {
            OrderType::Bid => match highest_bid {
                None => highest_bid = Some(entry),
                Some(h) if { entry.price > h.price } => highest_bid = Some(entry),
                _ => (),
            },
            OrderType::Ask => match lowest_ask {
                None => lowest_ask = Some(entry),
                Some(l) if { entry.price < l.price } => lowest_ask = Some(entry),
                _ => (),
            },
        }
    }

    OrderBookMiddle {
        highest_bid: highest_bid,
        lowest_ask: lowest_ask,
    }
}

pub fn update_orderbook(order_book: Option<OrderBook>, input: Value) -> Option<OrderBook> {
    // ["o", <1 for bid 0 for ask>, "<price>", "<size>", "<epoch_ms>"]
    let bid: u8 = serde_json::from_value(input[1].clone()).unwrap();
    let price_s: String = serde_json::from_value(input[2].clone()).unwrap();
    let size_s: String = serde_json::from_value(input[3].clone()).unwrap();
    let price: f64 = price_s.parse::<f64>().unwrap();
    let size: f64 = size_s.parse::<f64>().unwrap();
    let order_type: OrderType = match bid {
        1 => OrderType::Bid,
        _ => OrderType::Ask,
    };
    let new_val = if size < F64_EPSILON {
        None
    } else {
        Some(OrderBookEntry {
            order_type: order_type,
            price: price,
            size: size,
        })
    };

    match order_book {
        None => None,
        Some(ob) => {
            let mut new_ob = ob.clone();
            match new_val {
                None => new_ob.remove(&price_s),
                Some(val) => new_ob.insert(price_s, val),
            };
            Some(new_ob)
        }
    }
}

pub fn parse_orderbook(input: Value) -> Option<OrderBook> {
    let parsed: PoloniexOrderBook = serde_json::from_value(input).unwrap();
    let mut ret: HashMap<String, OrderBookEntry> = HashMap::new();

    for (price_s, size_s) in parsed.order_book.0 {
        let price: f64 = price_s.parse::<f64>().unwrap();
        let size: f64 = size_s.parse::<f64>().unwrap();
        ret.insert(
            price_s,
            OrderBookEntry {
                order_type: OrderType::Ask,
                price: price,
                size: size,
            },
        );
    }

    for (price_s, size_s) in parsed.order_book.1 {
        let price: f64 = price_s.parse::<f64>().unwrap();
        let size: f64 = size_s.parse::<f64>().unwrap();
        ret.insert(
            price_s,
            OrderBookEntry {
                order_type: OrderType::Bid,
                price: price,
                size: size,
            },
        );
    }

    Some(ret)
}

#[cfg(test)]
mod tests {
    use super::*;

    const INPUT: &str = r#"{
        "currencyPair": "USDT_LTC",
        "orderBook": [
          {
            "123.71470735": "0.80831133",
            "123.87038423": "72.80000000",
            "123.92495682": "37.09637200",
            "123.96200000": "4.90000000",
            "1235.00000000": "17.60000000",
            "1235.20000000": "163.08933013",
            "124.04400000": "12.90000000",
            "999.77707823": "15.70192415"
          },
          {
            "0.00000001": "6164523.62636999",
            "0.00000002": "11524758480.50000000",
            "0.00000009": "22222222.22222222",
            "123.15153929": "145.70000000",
            "123.15200000": "25.50000000",
            "123.29200001": "50.00000000",
            "123.39300000": "12.90000000",
            "123.42600000": "4.90000000",
            "123.54238195": "72.80000000",
            "123.58845138": "34.09055384",
            "123.61432906": "15.00000000",
            "123.61626624": "8.36335043",
            "123.61626625": "30.54540138"
          }
        ]
      }"#;

    #[test]
    fn parse_orderbook_test() {
        let res = parse_orderbook(INPUT.to_string());
        let expected = HashMap::from([
            (
                "123.61626625".to_string(),
                OrderBookEntry {
                    order_type: OrderType::Bid,
                    size: 30.54540138,
                    price: 123.61626625,
                },
            ),
            (
                "1235.20000000".to_string(),
                OrderBookEntry {
                    order_type: OrderType::Ask,
                    size: 163.08933013,
                    price: 1235.2,
                },
            ),
            (
                "123.15153929".to_string(),
                OrderBookEntry {
                    order_type: OrderType::Bid,
                    size: 145.7,
                    price: 123.15153929,
                },
            ),
            (
                "999.77707823".to_string(),
                OrderBookEntry {
                    order_type: OrderType::Ask,
                    size: 15.70192415,
                    price: 999.77707823,
                },
            ),
            (
                "123.71470735".to_string(),
                OrderBookEntry {
                    order_type: OrderType::Ask,
                    size: 0.80831133,
                    price: 123.71470735,
                },
            ),
            (
                "123.29200001".to_string(),
                OrderBookEntry {
                    order_type: OrderType::Bid,
                    size: 50.0,
                    price: 123.29200001,
                },
            ),
            (
                "123.39300000".to_string(),
                OrderBookEntry {
                    order_type: OrderType::Bid,
                    size: 12.9,
                    price: 123.393,
                },
            ),
            (
                "123.61626624".to_string(),
                OrderBookEntry {
                    order_type: OrderType::Bid,
                    size: 8.36335043,
                    price: 123.61626624,
                },
            ),
            (
                "123.96200000".to_string(),
                OrderBookEntry {
                    order_type: OrderType::Ask,
                    size: 4.9,
                    price: 123.962,
                },
            ),
            (
                "123.54238195".to_string(),
                OrderBookEntry {
                    order_type: OrderType::Bid,
                    size: 72.8,
                    price: 123.54238195,
                },
            ),
            (
                "123.58845138".to_string(),
                OrderBookEntry {
                    order_type: OrderType::Bid,
                    size: 34.09055384,
                    price: 123.58845138,
                },
            ),
            (
                "0.00000002".to_string(),
                OrderBookEntry {
                    order_type: OrderType::Bid,
                    size: 11524758480.5,
                    price: 0.00000002,
                },
            ),
            (
                "123.15200000".to_string(),
                OrderBookEntry {
                    order_type: OrderType::Bid,
                    size: 25.5,
                    price: 123.152,
                },
            ),
            (
                "123.87038423".to_string(),
                OrderBookEntry {
                    order_type: OrderType::Ask,
                    size: 72.8,
                    price: 123.87038423,
                },
            ),
            (
                "0.00000001".to_string(),
                OrderBookEntry {
                    order_type: OrderType::Bid,
                    size: 6164523.62636999,
                    price: 0.00000001,
                },
            ),
            (
                "123.92495682".to_string(),
                OrderBookEntry {
                    order_type: OrderType::Ask,
                    size: 37.096372,
                    price: 123.92495682,
                },
            ),
            (
                "123.61432906".to_string(),
                OrderBookEntry {
                    order_type: OrderType::Bid,
                    size: 15.0,
                    price: 123.61432906,
                },
            ),
            (
                "1235.00000000".to_string(),
                OrderBookEntry {
                    order_type: OrderType::Ask,
                    size: 17.6,
                    price: 1235.0,
                },
            ),
            (
                "0.00000009".to_string(),
                OrderBookEntry {
                    order_type: OrderType::Bid,
                    size: 22222222.22222222,
                    price: 0.00000009,
                },
            ),
            (
                "123.42600000".to_string(),
                OrderBookEntry {
                    order_type: OrderType::Bid,
                    size: 4.9,
                    price: 123.426,
                },
            ),
            (
                "124.04400000".to_string(),
                OrderBookEntry {
                    order_type: OrderType::Ask,
                    size: 12.9,
                    price: 124.044,
                },
            ),
        ]);

        assert_eq!(res, expected);
    }
}
