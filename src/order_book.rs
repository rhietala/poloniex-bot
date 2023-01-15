use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use tungstenite::{connect, Message};
use url::Url;

use crate::diesel::prelude::*;
use crate::models::*;

const API_URL: &str = "wss://api2.poloniex.com";

pub const HEARTBEAT_ID: u32 = 1010;
const F64_EPSILON: f64 = 1e-10;

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
    price: f64,
}

pub type OrderBook = HashMap<String, OrderBookEntry>;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct OrderBookMiddle {
    highest_bid: Option<OrderBookEntry>,
    lowest_ask: Option<OrderBookEntry>,
}

#[derive(Serialize, Deserialize, Debug)]
struct Command {
    command: String,
    channel: String,
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

pub fn do_buy(
    connection: &mut PgConnection,
    trade: &Trade,
    lowest_ask: OrderBookEntry,
) -> Result<Option<Trade>, Box<dyn std::error::Error>> {
    use crate::schema::trades::dsl::*;

    let new_open: f32 = lowest_ask.price as f32;
    let updated_trade = diesel::update(trade)
        .set((open_at.eq(Utc::now()), open.eq(Some(new_open))))
        .get_result::<Trade>(connection)
        .unwrap();

    Ok(Some(updated_trade))
}

pub fn do_trade(connection: &mut PgConnection, trade_id: i32) {
    use crate::schema::trades::dsl::*;
    let (mut socket, _response) = connect(Url::parse(API_URL).unwrap()).expect("Can't connect");

    // fetch trade by id
    let trade: Trade = trades.find(trade_id).first(connection).unwrap();

    let subscribe_command = Command {
        command: "subscribe".to_string(),
        channel: format!("USDT_{}", trade.quote).to_string(),
    };

    socket
        .write_message(Message::Text(
            serde_json::to_string(&subscribe_command).unwrap(),
        ))
        .unwrap();

    let mut channel_id: Option<u32> = None;
    let mut order_book: Option<OrderBook> = None;
    let mut buy_value: Option<f32> = None;
    let mut prev_highest_bid: Option<f64> = None;
    let mut continue_trade: bool = true;

    loop {
        let msg_s = socket.read_message().expect("Error reading message");
        let parsed = parse_message(msg_s.to_string());

        if channel_id == None && parsed.channel_id != HEARTBEAT_ID {
            channel_id = Some(parsed.channel_id);
        }

        if channel_id == Some(parsed.channel_id) {
            for msg in parsed.messages.into_iter() {
                let ret = do_message(
                    connection,
                    &trade,
                    msg,
                    order_book,
                    buy_value,
                    prev_highest_bid,
                );
                continue_trade = ret.0;
                order_book = ret.1;
                buy_value = ret.2;
                prev_highest_bid = ret.3;

                // don't overwrite continue_trade with possible other messages in the
                // same batch
                if !continue_trade {
                    break;
                }
            }
        }
        if !continue_trade && order_book == None {
            // delete the trade that was never started
            use crate::schema::trades::dsl::*;
            diesel::delete(trades.filter(id.eq(trade.id)))
                .execute(connection)
                .unwrap();
        }
        if !continue_trade {
            break;
        }
    }
    // socket.close(None)
}

fn check_sell(
    connection: &mut PgConnection,
    trade: &Trade,
    highest_bid: OrderBookEntry,
) -> Result<bool, Box<dyn std::error::Error>> {
    use crate::schema::trades::dsl::*;

    let rows = trades
        .filter(id.eq(trade.id))
        .limit(1)
        .load::<Trade>(connection)
        .unwrap();
    let current_trade = rows.get(0).unwrap();

    // close trade if current bid is below target,
    // or take profit if the bid is at +5% from target
    // (inside the same candle)
    if current_trade.target as f64 > highest_bid.price
        || highest_bid.price / current_trade.target as f64 > 1.05
    {
        println!(
            "{} closing trade at {}: {:.1}%",
            current_trade.quote,
            highest_bid.price,
            ((highest_bid.price / f64::from(current_trade.open.unwrap())) - 1.0) * 100.0
        );

        diesel::update(trade)
            .set((
                close_at.eq(Utc::now()),
                close.eq(Some(highest_bid.price as f32)),
            ))
            .execute(connection)
            .unwrap();

        return Ok(false);
    }
    Ok(true)
}

fn do_message(
    connection: &mut PgConnection,
    trade: &Trade,
    msg: Value,
    mut order_book: Option<OrderBook>,
    mut buy_value: Option<f32>,
    mut prev_highest_bid: Option<f64>,
) -> (bool, Option<OrderBook>, Option<f32>, Option<f64>) {
    let command: String = serde_json::from_value(msg[0].clone()).unwrap();
    order_book = match command.as_str() {
        // update whole order book
        "i" => parse_orderbook(msg[1].clone()),
        "o" => update_orderbook(order_book, msg),
        _ => order_book,
    };
    match order_book.clone() {
        Some(ob) => match (find_middle(ob), buy_value, prev_highest_bid) {
            // first loop round
            (
                OrderBookMiddle {
                    highest_bid: Some(highest_bid),
                    lowest_ask: Some(lowest_ask),
                },
                None,
                _,
            ) => {
                // if highest bid is below the target, don't start trade
                if highest_bid.price < trade.target.into() {
                    println!(
                        "{} highest bid ({:?}) below target ({:?}) => no action",
                        trade.quote, highest_bid.price, trade.target
                    );

                    return (false, None, None, None);
                }

                // if highest bid is too high compared to target, don't start trade
                if (highest_bid.price - f64::from(trade.target)) / f64::from(trade.target) > 0.02 {
                    println!(
                        "{} highest bid ({:?}) too high above target ({:?}) => no action",
                        trade.quote, highest_bid.price, trade.target
                    );

                    return (false, None, None, None);
                }

                let buy_trade = do_buy(connection, trade, lowest_ask).unwrap();
                match buy_trade {
                    Some(bt) => {
                        println!("{} buy at {}", bt.quote, bt.open.unwrap());
                        buy_value = bt.open
                    }
                    None => (),
                }
                prev_highest_bid = Some(highest_bid.price)
            }
            (
                OrderBookMiddle {
                    highest_bid: Some(highest_bid),
                    lowest_ask: _,
                },
                Some(buy_value),
                Some(phb),
            ) if (phb - highest_bid.price).abs() > F64_EPSILON => {
                println!(
                    "{} highest bid: {}, trade at {:.1}%",
                    trade.quote,
                    highest_bid.price,
                    ((highest_bid.price / f64::from(buy_value)) - 1.0) * 100.0
                );
                prev_highest_bid = Some(highest_bid.price);

                let continue_trade = check_sell(connection, trade, highest_bid).unwrap();
                if !continue_trade {
                    return (false, order_book, Some(buy_value), prev_highest_bid);
                }
            }
            _ => (),
        },
        _ => (),
    };

    (true, order_book, buy_value, prev_highest_bid)
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
