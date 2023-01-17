use chrono::Utc;
use serde_json::Value;
use tungstenite::{connect, Message};
use url::Url;

use crate::diesel::prelude::*;
use crate::models::*;

use crate::order_book::*;

const API_URL: &str = "wss://api2.poloniex.com";

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

fn do_buy(
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
