-- Full-text search over events. FTS5 is a build-time property of the vendored
-- SQLite (D-11): it was measured present, a startup assertion fails loudly if
-- a dependency bump ever removes it, and this migration therefore has exactly
-- one body. There is no runtime branch and no second schema.
--
-- External-content: the index stores no copy of the text, it reads the events
-- row by rowid when it needs one. Two triggers keep it in step with inserts
-- and deletes; events are never updated, so there is no third.
CREATE VIRTUAL TABLE events_fts USING fts5(
    entity_display_name,
    user_friendly_summary,
    technical_summary,
    entity_id,
    content='events',
    content_rowid='id'
);

CREATE TRIGGER events_fts_after_insert AFTER INSERT ON events BEGIN
    INSERT INTO events_fts (rowid, entity_display_name, user_friendly_summary, technical_summary, entity_id)
    VALUES (new.id, new.entity_display_name, new.user_friendly_summary, new.technical_summary, new.entity_id);
END;

CREATE TRIGGER events_fts_after_delete AFTER DELETE ON events BEGIN
    INSERT INTO events_fts (events_fts, rowid, entity_display_name, user_friendly_summary, technical_summary, entity_id)
    VALUES ('delete', old.id, old.entity_display_name, old.user_friendly_summary, old.technical_summary, old.entity_id);
END;

-- Index whatever was written before this migration existed.
INSERT INTO events_fts (events_fts) VALUES ('rebuild');
