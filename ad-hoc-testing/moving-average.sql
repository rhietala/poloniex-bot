SELECT
  raw.timestamp,
  raw.average - raw.ma10 as diff,
  raw.average,
  raw.ma10,
  raw.ma30,
  raw.ma200,
  raw.ma10 > raw.ma30 AND raw.ma30 > raw.ma200 as f
FROM
  (
    SELECT
      timestamp,
      quote,
      average,
      AVG(average) OVER(
        ORDER BY
          timestamp ROWS BETWEEN 10 PRECEDING
          AND CURRENT ROW
      ) AS ma10,
      AVG(average) OVER(
        ORDER BY
          timestamp ROWS BETWEEN 30 PRECEDING
          AND CURRENT ROW
      ) AS ma30,
      AVG(average) OVER(
        ORDER BY
          timestamp ROWS BETWEEN 200 PRECEDING
          AND CURRENT ROW
      ) AS ma200
    FROM
      candles
    WHERE
      base = 'USDT'
      AND quote = 'LRC'
      AND period = 900
    ORDER BY
      timestamp DESC
    LIMIT
      1
  ) as raw;


--

SELECT
  DISTINCT ON (quote) quote,
  timestamp,
  AVG(average) OVER(
    PARTITION BY quote, base, period
    ORDER BY
      timestamp ROWS BETWEEN 10 PRECEDING
      AND CURRENT ROW
  ) AS ma10,
  AVG(average) OVER(
    PARTITION BY quote, base, period
    ORDER BY
      timestamp ROWS BETWEEN 30 PRECEDING
      AND CURRENT ROW
  ) AS ma30,
  AVG(average) OVER(
    PARTITION BY quote, base, period
    ORDER BY
      timestamp ROWS BETWEEN 200 PRECEDING
      AND CURRENT ROW
  ) AS ma200
FROM
  candles
ORDER BY quote, timestamp DESC;

--

SELECT
  raw.quote,
  raw.average / raw.ma30 as rise,
  raw.average,
  raw.ma10,
  raw.ma30,
  raw.ma200,
  raw.average > raw.ma10 AND raw.ma10 > raw.ma30 AND raw.ma30 > raw.ma200 as NOUSEEKO
FROM
  (
SELECT
  DISTINCT ON (quote) quote,
  timestamp,
  average,
  AVG(average) OVER(
    PARTITION BY quote,
    base,
    period
    ORDER BY
      timestamp ROWS BETWEEN 10 PRECEDING
      AND CURRENT ROW
  ) AS ma10,
  AVG(average) OVER(
    PARTITION BY quote,
    base,
    period
    ORDER BY
      timestamp ROWS BETWEEN 30 PRECEDING
      AND CURRENT ROW
  ) AS ma30,
  AVG(average) OVER(
    PARTITION BY quote,
    base,
    period
    ORDER BY
      timestamp ROWS BETWEEN 200 PRECEDING
      AND CURRENT ROW
  ) AS ma200
FROM
  candles
WHERE
  base = 'USDT'
  AND period = 900
  AND quote IN (
    SELECT
      quote
    FROM
      candles
    WHERE
      base = 'USDT'
      AND period = 900
      AND timestamp > (current_timestamp - interval '24 hours')
    GROUP BY
      quote,
      base,
      period
    HAVING
      SUM(volume) > 3000
      AND MAX(timestamp) > (current_timestamp - interval '30 minutes')
  )
ORDER BY
  quote,
  timestamp DESC) as raw
WHERE (raw.average > raw.ma10 AND raw.ma10 > raw.ma30 AND raw.ma30 > raw.ma200) is true
ORDER BY raw.average / raw.ma30 DESC;


-- CTE

-- filter symbols so that included must have
-- - a row in the previous 30 minutes
-- - previous 24 hour volume above 3000 USDT
WITH filtered_symbols AS (
  SELECT
    quote
  FROM
    candles
  WHERE
    base = 'USDT'
    AND period = 900
    AND timestamp > (current_timestamp - interval '24 hours')
  GROUP BY
    quote,
    base,
    period
  HAVING
    SUM(volume) > 3000
    AND MAX(timestamp) > (current_timestamp - interval '30 minutes')
),
analyzed AS (
  SELECT
    DISTINCT ON (quote) quote,
    timestamp,
    average,
    -- average over previous 10 rows
    AVG(average) OVER(
      PARTITION BY quote,
      base,
      period
      ORDER BY
        timestamp ROWS BETWEEN 10 PRECEDING
        AND CURRENT ROW
    ) AS ma10,
    -- average over previous 30 rows
    AVG(average) OVER(
      PARTITION BY quote,
      base,
      period
      ORDER BY
        timestamp ROWS BETWEEN 30 PRECEDING
        AND CURRENT ROW
    ) AS ma30,
    -- average over previous 200 rows
    AVG(average) OVER(
      PARTITION BY quote,
      base,
      period
      ORDER BY
        timestamp ROWS BETWEEN 200 PRECEDING
        AND CURRENT ROW
    ) AS ma200
  FROM
    candles
  WHERE
    base = 'USDT'
    AND period = 900
    AND quote IN (SELECT quote FROM filtered_symbols)
  ORDER BY
    quote,
    timestamp DESC
)
INSERT INTO shortlist(quote, timestamp, average, confidence) (SELECT
  quote,
  NOW(),
  average,
  average / ma30 as confidence
FROM
  (SELECT * FROM analyzed) AS analyzed
WHERE
  -- actual logic: current value must be above 10-period moving average,
  -- which must be above 30-period MA, which must be above 200-period MA
  (
    average > ma10
    AND ma10 > ma30
    AND ma30 > ma200
  ) is true);


--

SELECT * FROM shortlist ORDER BY confidence;

--

DELETE FROM shortlist;