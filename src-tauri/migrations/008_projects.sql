-- User-approved development-project roots are independent from watched
-- folders: the same canonical directory can be both without collapsing two
-- separate consent choices into one row.
CREATE TABLE projects (
    id       TEXT    PRIMARY KEY,
    path     TEXT    NOT NULL COLLATE NOCASE UNIQUE,
    name     TEXT    NOT NULL,
    added_at INTEGER NOT NULL
);

CREATE INDEX idx_projects_added_at ON projects (added_at, id);

-- Project/process evidence is read from process start events. This partial
-- expression index keeps the exact-path proof bounded without copying raw
-- executable paths into a second table.
CREATE INDEX idx_events_process_started_path
    ON events (canonical_path(json_extract(metadata_json, '$.path')), entity_id, timestamp DESC)
    WHERE source_collector = 'process'
      AND event_type = 'started'
      AND entity_type = 'process'
      AND json_extract(metadata_json, '$.path') IS NOT NULL;

-- Cleanup enters retained observations by one relationship target, avoiding
-- repeated history scans while checking the remaining registered roots.
CREATE INDEX idx_events_process_started_entity_path
    ON events (entity_id, canonical_path(json_extract(metadata_json, '$.path')))
    WHERE source_collector = 'process'
      AND event_type = 'started'
      AND entity_type = 'process'
      AND json_extract(metadata_json, '$.path') IS NOT NULL;
