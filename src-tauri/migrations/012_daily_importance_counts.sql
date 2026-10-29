-- Exact dashboard counts without rescanning every historical event.
-- Timestamps are epoch milliseconds; one aggregate row represents one UTC
-- day. The summary query still scans both boundary days for arbitrary ranges
-- and reads only complete interior days from this table.
CREATE TABLE event_importance_daily (
    day_start      INTEGER PRIMARY KEY,
    informational  INTEGER NOT NULL DEFAULT 0,
    notable        INTEGER NOT NULL DEFAULT 0,
    important      INTEGER NOT NULL DEFAULT 0,
    critical       INTEGER NOT NULL DEFAULT 0
);

CREATE TRIGGER event_importance_daily_after_insert AFTER INSERT ON events BEGIN
    INSERT INTO event_importance_daily (
        day_start, informational, notable, important, critical
    )
    VALUES (
        (new.timestamp / 86400000) * 86400000,
        CASE WHEN new.importance = 'informational' THEN 1 ELSE 0 END,
        CASE WHEN new.importance = 'notable' THEN 1 ELSE 0 END,
        CASE WHEN new.importance = 'important' THEN 1 ELSE 0 END,
        CASE WHEN new.importance = 'critical' THEN 1 ELSE 0 END
    )
    ON CONFLICT(day_start) DO UPDATE SET
        informational = informational + excluded.informational,
        notable = notable + excluded.notable,
        important = important + excluded.important,
        critical = critical + excluded.critical;
END;

CREATE TRIGGER event_importance_daily_after_delete AFTER DELETE ON events BEGIN
    UPDATE event_importance_daily
    SET informational = informational - CASE WHEN old.importance = 'informational' THEN 1 ELSE 0 END,
        notable = notable - CASE WHEN old.importance = 'notable' THEN 1 ELSE 0 END,
        important = important - CASE WHEN old.importance = 'important' THEN 1 ELSE 0 END,
        critical = critical - CASE WHEN old.importance = 'critical' THEN 1 ELSE 0 END
    WHERE day_start = (old.timestamp / 86400000) * 86400000;
    DELETE FROM event_importance_daily
    WHERE day_start = (old.timestamp / 86400000) * 86400000
      AND informational = 0 AND notable = 0 AND important = 0 AND critical = 0;
END;

-- Populate rows written before this migration existed.
INSERT INTO event_importance_daily (
    day_start, informational, notable, important, critical
)
SELECT
    (timestamp / 86400000) * 86400000,
    SUM(CASE WHEN importance = 'informational' THEN 1 ELSE 0 END),
    SUM(CASE WHEN importance = 'notable' THEN 1 ELSE 0 END),
    SUM(CASE WHEN importance = 'important' THEN 1 ELSE 0 END),
    SUM(CASE WHEN importance = 'critical' THEN 1 ELSE 0 END)
FROM events
GROUP BY (timestamp / 86400000) * 86400000;
