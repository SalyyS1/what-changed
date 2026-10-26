-- Correct the first partial UTC day without changing applied migration 013.
-- i64::MIN lies 25,975,808 ms before the next representable UTC midnight.
-- Rebuild the derived projection so databases already on v13 are repaired.
DROP TRIGGER event_importance_daily_after_insert;
DROP TRIGGER event_importance_daily_after_delete;

DELETE FROM event_importance_daily;

INSERT INTO event_importance_daily (
    day_start,
    informational,
    notable,
    important,
    critical
)
SELECT
    CASE
        WHEN timestamp <= -9223372036828800001
            THEN CAST('-9223372036854775808' AS INTEGER)
        ELSE (CASE
            WHEN timestamp < 0 AND timestamp % 86400000 != 0
                THEN (timestamp / 86400000) - 1
            ELSE timestamp / 86400000
        END) * 86400000
    END,
    SUM(CASE WHEN importance = 'informational' THEN 1 ELSE 0 END),
    SUM(CASE WHEN importance = 'notable' THEN 1 ELSE 0 END),
    SUM(CASE WHEN importance = 'important' THEN 1 ELSE 0 END),
    SUM(CASE WHEN importance = 'critical' THEN 1 ELSE 0 END)
FROM events
GROUP BY CASE
    WHEN timestamp <= -9223372036828800001
        THEN CAST('-9223372036854775808' AS INTEGER)
    ELSE (CASE
        WHEN timestamp < 0 AND timestamp % 86400000 != 0
            THEN (timestamp / 86400000) - 1
        ELSE timestamp / 86400000
    END) * 86400000
END;

CREATE TRIGGER event_importance_daily_after_insert AFTER INSERT ON events BEGIN
    INSERT INTO event_importance_daily (
        day_start,
        informational,
        notable,
        important,
        critical
    ) VALUES (
        CASE
            WHEN new.timestamp <= -9223372036828800001
                THEN CAST('-9223372036854775808' AS INTEGER)
            ELSE (CASE
                WHEN new.timestamp < 0 AND new.timestamp % 86400000 != 0
                    THEN (new.timestamp / 86400000) - 1
                ELSE new.timestamp / 86400000
            END) * 86400000
        END,
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
    SET
        informational = informational - CASE WHEN old.importance = 'informational' THEN 1 ELSE 0 END,
        notable = notable - CASE WHEN old.importance = 'notable' THEN 1 ELSE 0 END,
        important = important - CASE WHEN old.importance = 'important' THEN 1 ELSE 0 END,
        critical = critical - CASE WHEN old.importance = 'critical' THEN 1 ELSE 0 END
    WHERE day_start = CASE
        WHEN old.timestamp <= -9223372036828800001
            THEN CAST('-9223372036854775808' AS INTEGER)
        ELSE (CASE
            WHEN old.timestamp < 0 AND old.timestamp % 86400000 != 0
                THEN (old.timestamp / 86400000) - 1
            ELSE old.timestamp / 86400000
        END) * 86400000
    END;
    DELETE FROM event_importance_daily
    WHERE day_start = CASE
        WHEN old.timestamp <= -9223372036828800001
            THEN CAST('-9223372036854775808' AS INTEGER)
        ELSE (CASE
            WHEN old.timestamp < 0 AND old.timestamp % 86400000 != 0
                THEN (old.timestamp / 86400000) - 1
            ELSE old.timestamp / 86400000
        END) * 86400000
    END
      AND informational = 0
      AND notable = 0
      AND important = 0
      AND critical = 0;
END;
