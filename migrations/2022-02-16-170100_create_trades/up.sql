-- Your SQL goes here
CREATE TABLE trades (
  id SERIAL PRIMARY KEY NOT NULL,
  base VARCHAR(20) NOT NULL,
  quote VARCHAR(20) NOT NULL,
  open_at TIMESTAMPTZ NOT NULL,
  close_at TIMESTAMPTZ,
  updated_at TIMESTAMPTZ NOT NULL,
  open_average REAL NOT NULL,
  target REAL NOT NULL,
  open REAL,
  close REAL,
  highest_bid REAL
)
