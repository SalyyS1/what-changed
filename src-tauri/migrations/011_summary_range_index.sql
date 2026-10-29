-- Cover the dashboard's range counts in one timestamp-ordered scan.
-- The existing importance-first index serves filtered importance pages, but
-- the dashboard needs all four buckets for the same time range.
CREATE INDEX idx_events_timestamp_importance ON events (timestamp, importance);
