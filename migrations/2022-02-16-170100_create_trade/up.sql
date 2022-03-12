-- Your SQL goes here
CREATE TABLE trade (
  id SERIAL PRIMARY KEY NOT NULL,
  base VARCHAR(20) NOT NULL,
  quote VARCHAR(20) NOT NULL,
  open_at TIMESTAMPTZ NOT NULL,
  close_at TIMESTAMPTZ,
  open_average REAL NOT NULL,
  target REAL NOT NULL,
  open REAL,
  close REAL
)