-- Your SQL goes here
CREATE TABLE shortlist (
  quote VARCHAR(20) PRIMARY KEY NOT NULL,
  timestamp TIMESTAMPTZ NOT NULL,
  average REAL NOT NULL,
  target REAL NOT NULL,
  confidence REAL NOT NULL
)