use chrono::Utc;
use serde_json::Value;
use tungstenite::{connect, Message};
use url::Url;

use crate::diesel::prelude::*;
use crate::models::*;

use crate::order_book::*;

const API_URL: &str = "wss://api2.poloniex.com";

// allow trade to drop by this amount before closing
// also the start decisions are based on this
pub const STOP_LOSS: f64 = 0.005;

// start trade if lowest ask is this much above target at maximum
pub const START_ABOVE_TARGET: f64 = 0.015;

// when updating trades, increase target at least by this amount
pub const CONSTANT_RISE: f64 = 0.0025;

// when checking for for buying or selling, don't do either if
// spread (higest bid - lowest ask) is less than this
pub const MAX_SPREAD: f64 = 0.0025;

pub fn do_trade(
    connection: &mut PgConnection,
    trade_id: i32,
) -> Result<(), Box<dyn std::error::Error>> {
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
    let mut buy_value: Option<f32> = trade.open;
    let mut prev_highest_bid: Option<f64> = trade.highest_bid.map(|x| x as f64);
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
                )?;
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
            break Ok(());
        }
    }
    // socket.close(None)
}

fn do_buy(
    connection: &mut PgConnection,
    trade: &Trade,
    lowest_ask: f32,
) -> Result<Trade, Box<diesel::result::Error>> {
    use crate::schema::trades::dsl::*;

    // the previous target comes from candles and is not that
    // real-time, set it based on stoploss and start to rise
    // from there
    let new_target: f32 = lowest_ask * (1.0 - STOP_LOSS as f32);

    diesel::update(trade)
        .set((
            open_at.eq(Utc::now()),
            open.eq(Some(lowest_ask)),
            target.eq(new_target),
        ))
        .get_result(connection)
        .map_err(|e| Box::new(e))
}

fn check_sell(
    connection: &mut PgConnection,
    trade: &Trade,
    highest_bid_ob: OrderBookEntry,
    lowest_ask_ob: OrderBookEntry,
) -> Result<bool, Box<dyn std::error::Error>> {
    use crate::schema::trades::dsl::*;

    let current_trade: Trade = trades.find(trade.id).first(connection)?;

    let tgt: f32 = current_trade.target;
    let cur: f32 = highest_bid_ob.price as f32;
    let spread: f64 = (lowest_ask_ob.price - highest_bid_ob.price) / lowest_ask_ob.price;

    // close trade if current bid is below target
    if cur < tgt {
        // if the order book has too high spread, don't hurry to sell
        if spread > MAX_SPREAD {
            log_trade(trade, format!("spread too high, not selling, {}", spread));
            return Ok(true);
        }

        let cur_open: f32 = current_trade.open.unwrap();

        log_trade(
            &current_trade,
            format!(
                "closing trade, close: {:?}, open: {:?}: {:.3}%",
                cur,
                cur_open,
                cur / cur_open
            ),
        );

        diesel::update(trade)
            .set((close_at.eq(Utc::now()), close.eq(Some(cur))))
            .execute(connection)?;

        return Ok(false);
    }

    // update target if current bid is more than stop loss above target
    let take_profit_tgt = cur * (1.0 - STOP_LOSS) as f32;
    let new_target = if take_profit_tgt > tgt {
        take_profit_tgt
    } else {
        tgt
    };

    // update trade based on heartbeat so that we'll know if the websocket
    // connection is still alive
    diesel::update(trade)
        .set((
            updated_at.eq(Utc::now()),
            highest_bid.eq(Some(cur)),
            target.eq(new_target),
        ))
        .execute(connection)?;

    Ok(true)
}

fn check_start(
    connection: &mut PgConnection,
    trade: &Trade,
    highest_bid: f64,
    lowest_ask: f64,
) -> Result<(bool, Option<f64>), Box<dyn std::error::Error>> {
    let target: f64 = trade.target as f64;

    // if highest bid is below the target, don't start trade
    if highest_bid < target {
        log_trade_hb(trade, "won't start trade (too low)", highest_bid, target);
        return Ok((false, None));
    }

    // if highest bid is too high compared to target, don't start trade
    // something strange is happening
    if highest_bid / target > (1.0 + START_ABOVE_TARGET) {
        log_trade_hb(trade, "won't start trade (too high)", highest_bid, target);
        return Ok((false, None));
    }

    let spread: f64 = (lowest_ask - highest_bid) / highest_bid;
    if spread > MAX_SPREAD {
        log_trade(trade, format!("spread too high, not buying, {}", spread));
        return Ok((true, None));
    }

    let buy_trade = do_buy(connection, trade, lowest_ask as f32)?;

    log_trade_hb(&buy_trade, "starting trade", highest_bid, target);

    Ok((true, Some(highest_bid)))
}

fn do_message(
    connection: &mut PgConnection,
    trade: &Trade,
    msg: Value,
    mut order_book: Option<OrderBook>,
    mut buy_value: Option<f32>,
    mut prev_highest_bid: Option<f64>,
) -> Result<(bool, Option<OrderBook>, Option<f32>, Option<f64>), Box<dyn std::error::Error>> {
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
                let (ct, phb) =
                    check_start(connection, trade, highest_bid.price, lowest_ask.price)?;
                prev_highest_bid = phb;
                buy_value = Some(lowest_ask.price as f32);
                if phb == None {
                    return Ok((ct, None, None, None));
                }
            }
            (
                OrderBookMiddle {
                    highest_bid: Some(highest_bid),
                    lowest_ask: Some(lowest_ask),
                },
                Some(buy_value),
                Some(phb),
            ) if (phb - highest_bid.price).abs() > F64_EPSILON => {
                prev_highest_bid = Some(highest_bid.price);

                let continue_trade =
                    check_sell(connection, trade, highest_bid, lowest_ask).unwrap();
                if !continue_trade {
                    return Ok((false, order_book, Some(buy_value), prev_highest_bid));
                }
            }
            _ => (),
        },
        _ => (),
    };

    Ok((true, order_book, buy_value, prev_highest_bid))
}

pub fn log_trade(trade: &Trade, message: String) {
    println!("TRADE {}, {}: {}", trade.id, trade.quote, message);
}

pub fn log_trade_hb(trade: &Trade, message: &str, highest_bid: f64, target: f64) {
    log_trade(
        trade,
        format!(
            "{}, highest bid: {:?}, target: {:?}: {:.3}%",
            message,
            highest_bid,
            target,
            highest_bid / target
        ),
    );
}
