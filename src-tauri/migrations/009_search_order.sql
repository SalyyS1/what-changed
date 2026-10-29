-- Narrow, covering search-order projection.
--
-- FTS5 returns matching rowids, while the event row itself is intentionally
-- wide (JSON before/after/metadata fields). Joining that wide table just to
-- sort by importance and timestamp makes broad searches perform a random
-- lookup for every match. Keep the two ordering values beside the rowid so
-- the result can be ordered without touching the wide event payload; the
-- service hydrates only the bounded ids it returns afterwards.
CREATE TABLE event_search_order (
    event_id        INTEGER PRIMARY KEY REFERENCES events(id) ON DELETE CASCADE,
    importance_rank INTEGER NOT NULL,
    timestamp       INTEGER NOT NULL
);

CREATE INDEX idx_event_search_order_rank_timestamp
    ON event_search_order (importance_rank DESC, timestamp DESC, event_id DESC);

CREATE TRIGGER event_search_order_after_insert AFTER INSERT ON events BEGIN
    INSERT OR REPLACE INTO event_search_order (event_id, importance_rank, timestamp)
    VALUES (
        new.id,
        CASE new.importance
            WHEN 'critical' THEN 3
            WHEN 'important' THEN 2
            WHEN 'notable' THEN 1
            ELSE 0
        END,
        new.timestamp
    );
END;

CREATE TRIGGER event_search_order_after_delete AFTER DELETE ON events BEGIN
    DELETE FROM event_search_order WHERE event_id = old.id;
END;

-- Populate rows written before this migration existed.
INSERT INTO event_search_order (event_id, importance_rank, timestamp)
SELECT
    id,
    CASE importance
        WHEN 'critical' THEN 3
        WHEN 'important' THEN 2
        WHEN 'notable' THEN 1
        ELSE 0
    END,
    timestamp
FROM events;
