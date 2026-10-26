-- Exact privacy counts without a full events index scan on every screen load.
-- Triggers run in the same transaction as every insert, delete, or category
-- correction, including retention and privacy purges. Keep only nonzero rows
-- to match GROUP BY events semantics.
CREATE TABLE event_category_counts (
    category    TEXT PRIMARY KEY,
    event_count INTEGER NOT NULL CHECK (event_count >= 0)
);

CREATE TRIGGER event_category_counts_after_insert AFTER INSERT ON events BEGIN
    INSERT INTO event_category_counts (category, event_count)
    VALUES (new.category, 1)
    ON CONFLICT(category) DO UPDATE SET event_count = event_count + 1;
END;

CREATE TRIGGER event_category_counts_after_delete AFTER DELETE ON events BEGIN
    UPDATE event_category_counts
    SET event_count = event_count - 1
    WHERE category = old.category;
    DELETE FROM event_category_counts
    WHERE category = old.category AND event_count = 0;
END;

CREATE TRIGGER event_category_counts_after_category_update
AFTER UPDATE OF category ON events
WHEN old.category != new.category
BEGIN
    UPDATE event_category_counts
    SET event_count = event_count - 1
    WHERE category = old.category;
    DELETE FROM event_category_counts
    WHERE category = old.category AND event_count = 0;
    INSERT INTO event_category_counts (category, event_count)
    VALUES (new.category, 1)
    ON CONFLICT(category) DO UPDATE SET event_count = event_count + 1;
END;

-- One-time backfill for existing histories. The migration runner applies the
-- DDL and backfill atomically, so readers cannot observe an incomplete count.
INSERT INTO event_category_counts (category, event_count)
SELECT category, COUNT(*) FROM events GROUP BY category;
