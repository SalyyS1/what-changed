-- Numeric metadata index for values that must remain searchable as evidence.
-- Keep this separate from FTS5: numeric equality is exact and index-backed,
-- rather than text matching or a LIKE fallback.
CREATE TABLE event_search_metadata (
    event_id INTEGER PRIMARY KEY REFERENCES events(id) ON DELETE CASCADE,
    port     INTEGER NOT NULL
);

CREATE INDEX idx_event_search_metadata_port ON event_search_metadata (port, event_id);

CREATE TRIGGER event_search_metadata_after_insert AFTER INSERT ON events
WHEN json_type(new.metadata_json, '$.port') IN ('integer', 'real')
BEGIN
    INSERT OR REPLACE INTO event_search_metadata (event_id, port)
    VALUES (new.id, CAST(json_extract(new.metadata_json, '$.port') AS INTEGER));
END;

CREATE TRIGGER event_search_metadata_after_delete AFTER DELETE ON events BEGIN
    DELETE FROM event_search_metadata WHERE event_id = old.id;
END;

-- Populate rows written before this migration existed.
INSERT INTO event_search_metadata (event_id, port)
SELECT id, CAST(json_extract(metadata_json, '$.port') AS INTEGER)
FROM events
WHERE json_type(metadata_json, '$.port') IN ('integer', 'real');
