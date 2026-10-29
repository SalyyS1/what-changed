-- FTS5 projection with importance as an indexed token.
--
-- Search results are ordered by importance before timestamp.  Keeping the
-- importance value in the FTS index lets the query ask for the highest bucket
-- first and stop as soon as the bounded result limit is full, instead of
-- sorting every matching row in the corpus.
CREATE VIRTUAL TABLE events_search_fts USING fts5(
    entity_display_name,
    user_friendly_summary,
    technical_summary,
    entity_id,
    importance,
    content='events',
    content_rowid='id'
);

CREATE TRIGGER events_search_fts_after_insert AFTER INSERT ON events BEGIN
    INSERT INTO events_search_fts (
        rowid, entity_display_name, user_friendly_summary,
        technical_summary, entity_id, importance
    )
    VALUES (
        new.id, new.entity_display_name, new.user_friendly_summary,
        new.technical_summary, new.entity_id, new.importance
    );
END;

CREATE TRIGGER events_search_fts_after_delete AFTER DELETE ON events BEGIN
    INSERT INTO events_search_fts (
        events_search_fts, rowid, entity_display_name, user_friendly_summary,
        technical_summary, entity_id, importance
    )
    VALUES (
        'delete', old.id, old.entity_display_name, old.user_friendly_summary,
        old.technical_summary, old.entity_id, old.importance
    );
END;

-- Index whatever was written before this migration existed.
INSERT INTO events_search_fts (events_search_fts) VALUES ('rebuild');
