extern crate poloniex_bot;

use serde::{Deserialize, Serialize};
use serde_json;
use tungstenite::{connect, Message};
use url::Url;

use self::poloniex_bot::*;

const API_URL: &str = "wss://api2.poloniex.com";

#[derive(Serialize, Deserialize, Debug)]
struct Command {
    command: String,
    channel: String,
}

// enum OrderType { Bid, Ask }

// struct OrderBookEntry {
//     order_type: OrderType,
// }

// #[derive(Serialize, Deserialize, Debug)]
// struct OrderBook {
// }

fn main() {
    use self::order_book::{HEARTBEAT_ID, parse_message, parse_orderbook, OrderBook, update_orderbook};
    env_logger::init();

    let (mut socket, response) = connect(Url::parse(API_URL).unwrap()).expect("Can't connect");

    println!("Connected to the server");
    println!("Response HTTP code: {}", response.status());
    // println!("Response contains the following headers:");
    // for (ref header, _value) in response.headers() {
    //     println!("* {}", header);
    // }

    let subscribe_command = Command {
        command: "subscribe".to_string(),
        channel: "USDT_LTC".to_string(),
    };

    socket
        .write_message(Message::Text(
            serde_json::to_string(&subscribe_command).unwrap(),
        ))
        .unwrap();

    let mut channel_id: Option<u32> = None;
    let mut order_book: Option<OrderBook> = None;

    loop {
        let msg_s = socket.read_message().expect("Error reading message");
        println!("{:?}", msg_s);

        let parsed = parse_message(msg_s.to_string());

        if channel_id == None && parsed.channel_id != HEARTBEAT_ID {
            channel_id = Some(parsed.channel_id);
        }

        if channel_id == Some(parsed.channel_id) {
            for msg in parsed.messages.into_iter() {
                let command: String = serde_json::from_value(msg[0].clone()).unwrap();
                println!("command {}", command);
                order_book = match command.as_str() {
                    // update whole order book
                    "i" => parse_orderbook(msg[1].clone()),
                    "o" => update_orderbook(order_book, msg),
                    _ => order_book,
                }
            }
        }
        // println!("{:?}", order_book);
    }
    // socket.close(None);
}
