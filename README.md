# Poloniex-bot

A trading bot for cryptocurrencies listed in [Poloniex](https://poloniex.com/)
exchange.

This is work in progress, and has never traded any real money. Currently the
trading algorithm also is not profitable.

Nevertheless, I've used the project to get better coding Rust and also some raw
PostgreSQL.

## Algorithm

The idea is to find the most trending coins and bet on them.

1. From all the coins available in Poloniex, fetch the most accurate historical
   data: 15 minute open-high-low-close-volume
   [fetch_data.rs](src/bin/fetch_data.rs)
2. Do sanity check -filtering for the coins (high enough recent traded volume,
   recent data available) [ride_the_wave.rs](src/ride_the_wave.rs)
3. Analyze which coins are trending up (current value must be above 10-period moving
   average, which must be above 30-period MA, which again must be above 200-period MA)
   [ride_the_wave.rs](src/ride_the_wave.rs)
4. Add passing coins to shortlist, set sell targets from the most recent 10-period MA
   both to shortlisted and to ongoing trades
   [ride_the_wave.rs](src/ride_the_wave.rs)
5. Pick a coin for trading from the shortlist, check that there isn't currently
   an ongoing trade for that coin [select_trade.rs](src/bin/select_trade.rs)
6. Fetch up-to-date order book for the selected coin
   [order_book.rs](src/order_book.rs)
7. Check if the trading is above sell target and if it is, open trade (i.e. buy)
   [order_book.rs](src/order_book.rs)
8. Follow order book updates and if current value is below sell target, close trade
   (i.e. sell) [order_book.rs](src/order_book.rs)

## APIs used

- [returnChartData](https://docs.poloniex.com/#returnchartdata) REST API gives
  historical data for crypto coin prices.
- [Price Aggregated Book](https://docs.poloniex.com/#price-aggregated-book) Websocket
  API gives the current order book data, and updates to it.

## Devops

Project is built in Github Actions continuous integration, which also deploys
the built application to server.
Logic is in [GH Actions workflow file](.github/workflows).

Environment is set up with Ansible scripts under
[ansible](ansible).

Database migrations under [migrations](migrations) follow
[Diesel](https://diesel.rs/) migration scheme. Currently database migrations
are not included in ansible, and must be run manually with

```
diesel migration run
```

Operations are organized so that data fetching and analyzing is done with one
binary [fetch_data.rs](src/bin/fetch_data.rs) and after the analysis
is done, new single cryptocurrency is traded with another binary
[select_trade.rs](src/bin/select_trade.rs)

Both are started periodically with crontab and their shared state is in database.
