-- Your SQL goes here
CREATE TABLE candles (
  base VARCHAR(20),
  quote VARCHAR(20),
  period INTEGER,
  timestamp TIMESTAMPTZ,
  high REAL,
  low REAL,
  open REAL,
  close REAL,
  average REAL,
  volume REAL,
  PRIMARY KEY (base, quote, period, timestamp)
)