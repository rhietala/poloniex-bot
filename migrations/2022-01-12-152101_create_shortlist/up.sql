-- Your SQL goes here
CREATE TABLE shortlist (
  quote VARCHAR(20),
  timestamp TIMESTAMPTZ,
  average REAL,
  confidence REAL,
  PRIMARY KEY (quote)
)