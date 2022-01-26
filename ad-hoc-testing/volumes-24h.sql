SELECT
  quote,
  timestamp,
  volume
FROM
  candles
WHERE
  base = 'USDT'
  AND period = 900
  AND quote IN ('BTC', 'ETH', 'DOGE')
  AND timestamp > (current_timestamp - interval '2 hours');


-- return list of symbols that have recent rows and last 24 hours volume > 3000 (USDT)

SELECT
  quote
FROM
  candles
WHERE
  base = 'USDT'
  AND period = 900
  AND timestamp > (current_timestamp - interval '24 hours')
GROUP BY quote, base, period
HAVING SUM(volume) > 3000 AND MAX(timestamp) > (current_timestamp - interval '30 minutes');
