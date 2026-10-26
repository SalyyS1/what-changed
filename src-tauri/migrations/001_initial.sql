-- The schema. Sixteen tables: the thirteen the brief mandates, plus three that
-- later work persists to and that would otherwise be created by nobody. Every
-- table the product will ever write to is created here or by a later numbered
-- migration in this directory; a build gate asserts that any table named in
-- Rust has a CREATE TABLE somewhere under migrations/.
--
-- Timestamps are epoch milliseconds UTC. Enum columns hold snake_case words so
-- a row reads in any SQLite browser. JSON columns are named *_json.

-- One row per change. Immutable once written; retention deletes, nothing
-- updates. `occurred_at_is_uncertain` is provenance: a change discovered by
-- diffing a persisted baseline after the app was closed has a detection time
-- in `timestamp` and no known occurrence time.
CREATE TABLE events (
    id                       INTEGER PRIMARY KEY,
    timestamp                INTEGER NOT NULL,
    category                 TEXT    NOT NULL,
    event_type               TEXT    NOT NULL,
    entity_type              TEXT    NOT NULL,
    entity_id                TEXT    NOT NULL,
    entity_display_name      TEXT    NOT NULL,
    severity                 TEXT    NOT NULL,
    importance               TEXT    NOT NULL,
    source_collector         TEXT    NOT NULL,
    before_json              TEXT,
    after_json               TEXT,
    metadata_json            TEXT,
    related_entity_ids       TEXT    NOT NULL DEFAULT '[]',
    user_friendly_summary    TEXT    NOT NULL,
    technical_summary        TEXT    NOT NULL,
    confidence               TEXT    NOT NULL,
    sensitive                INTEGER NOT NULL DEFAULT 0,
    occurred_at_is_uncertain INTEGER NOT NULL DEFAULT 0
);

-- The things events are about. The id follows the grammar in domain/ids.rs so
-- the same service or file is the same row across restarts.
CREATE TABLE entities (
    id            TEXT    PRIMARY KEY,
    entity_type   TEXT    NOT NULL,
    display_name  TEXT    NOT NULL,
    first_seen    INTEGER NOT NULL,
    last_seen     INTEGER NOT NULL,
    metadata_json TEXT
);

-- Proven edges between entities. The event that recorded an edge is kept as
-- evidence and set to NULL when that event expires: the edge outlives it.
-- Deleting an entity removes its edges so the graph cannot grow orphans.
CREATE TABLE entity_relationships (
    id          INTEGER PRIMARY KEY,
    source_id   TEXT    NOT NULL REFERENCES entities(id) ON DELETE CASCADE,
    target_id   TEXT    NOT NULL REFERENCES entities(id) ON DELETE CASCADE,
    kind        TEXT    NOT NULL,
    event_id    INTEGER REFERENCES events(id) ON DELETE SET NULL,
    recorded_at INTEGER NOT NULL,
    UNIQUE (source_id, target_id, kind)
);

-- Known-good markers a person creates while things work.
CREATE TABLE checkpoints (
    id                 INTEGER PRIMARY KEY,
    name               TEXT    NOT NULL,
    kind               TEXT    NOT NULL DEFAULT 'manual',
    reason             TEXT,
    created_at         INTEGER NOT NULL,
    item_count         INTEGER NOT NULL DEFAULT 0,
    duration_ms        INTEGER NOT NULL DEFAULT 0,
    partial            INTEGER NOT NULL DEFAULT 0,
    missing_collectors TEXT    NOT NULL DEFAULT '[]'
);

-- A collector's state set at one moment. A snapshot that belongs to a
-- checkpoint is exempt from retention pruning and goes when the checkpoint
-- goes; one with no checkpoint expires on the snapshot lifetime.
CREATE TABLE snapshots (
    id                 INTEGER PRIMARY KEY,
    collector_id       TEXT    NOT NULL,
    entity_kind        TEXT    NOT NULL,
    entity_external_id TEXT,
    entity_name        TEXT,
    state_json         TEXT    NOT NULL,
    captured_at        INTEGER NOT NULL,
    checkpoint_id      INTEGER REFERENCES checkpoints(id) ON DELETE CASCADE
);

-- Reserved for health checks, a deferred feature. Created now so adding the
-- feature later needs no migration. Unused until then, and documented as such.
CREATE TABLE health_states (
    id           INTEGER PRIMARY KEY,
    check_id     TEXT    NOT NULL,
    state        TEXT    NOT NULL,
    observed_at  INTEGER NOT NULL,
    details_json TEXT
);

-- One row per collector, created the first time the runtime reports on it.
CREATE TABLE collector_status (
    collector_id         TEXT    PRIMARY KEY,
    enabled              INTEGER NOT NULL DEFAULT 1,
    state                TEXT    NOT NULL DEFAULT 'idle',
    last_run_at          INTEGER,
    last_duration_ms     INTEGER,
    events_emitted       INTEGER NOT NULL DEFAULT 0,
    consecutive_failures INTEGER NOT NULL DEFAULT 0,
    last_error           TEXT,
    updated_at           INTEGER NOT NULL
);

-- Key/value settings; every value is JSON.
CREATE TABLE settings (
    key        TEXT    PRIMARY KEY,
    value      TEXT    NOT NULL,
    updated_at INTEGER NOT NULL
);

-- Folders the user chose to watch, and registered project roots.
CREATE TABLE watched_locations (
    id        INTEGER PRIMARY KEY,
    path      TEXT    NOT NULL UNIQUE,
    kind      TEXT    NOT NULL DEFAULT 'folder',
    added_at  INTEGER NOT NULL,
    enabled   INTEGER NOT NULL DEFAULT 1,
    max_depth INTEGER NOT NULL DEFAULT 6
);

-- Installed applications as last enumerated, so an update can be described
-- as a version change rather than as an appearance.
CREATE TABLE applications (
    id               TEXT    PRIMARY KEY,
    display_name     TEXT    NOT NULL,
    version          TEXT,
    publisher        TEXT,
    install_date     TEXT,
    install_location TEXT,
    hive             TEXT    NOT NULL,
    registry_key     TEXT    NOT NULL,
    first_seen       INTEGER NOT NULL,
    last_seen        INTEGER NOT NULL,
    removed_at       INTEGER
);

-- The denormalized investigation history the UI pages.
CREATE TABLE investigations (
    id                     INTEGER PRIMARY KEY,
    problem                TEXT    NOT NULL,
    window_json            TEXT    NOT NULL,
    resolved_from          INTEGER NOT NULL,
    resolved_to            INTEGER NOT NULL,
    suspects_json          TEXT    NOT NULL,
    total_events_in_window INTEGER NOT NULL,
    created_at             INTEGER NOT NULL
);

-- One row per analysis run, keyed to the investigation it served, with the
-- weights it ran under so a past ranking stays auditable after a weight change.
CREATE TABLE analysis_sessions (
    id                INTEGER PRIMARY KEY,
    investigation_id  INTEGER NOT NULL REFERENCES investigations(id) ON DELETE CASCADE,
    started_at        INTEGER NOT NULL,
    finished_at       INTEGER,
    weights_json      TEXT    NOT NULL,
    events_considered INTEGER NOT NULL DEFAULT 0,
    downsampled       INTEGER NOT NULL DEFAULT 0
);

-- One row per ranked suspect with every score term kept separately.
CREATE TABLE analysis_results (
    id               INTEGER PRIMARY KEY,
    session_id       INTEGER NOT NULL REFERENCES analysis_sessions(id) ON DELETE CASCADE,
    event_id         INTEGER,
    rank             INTEGER NOT NULL,
    score            REAL    NOT NULL,
    score_terms_json TEXT    NOT NULL,
    confidence_class TEXT    NOT NULL,
    chain_json       TEXT
);

-- The migration ledger: which versions were applied, when, and the SHA-256 of
-- the SQL that was applied, so an edited migration is detected rather than
-- silently trusted.
CREATE TABLE schema_migrations (
    version    INTEGER PRIMARY KEY,
    name       TEXT    NOT NULL,
    applied_at INTEGER NOT NULL,
    sha256     TEXT    NOT NULL
);

-- Each collector's last-known state. On the first cycle after a restart a
-- collector diffs against this instead of treating the current state as a
-- fresh baseline, so a change made while the app was closed is reported.
-- One row per (collector, kind), so the table is bounded by collectors times
-- kinds, not by entities.
CREATE TABLE collector_baselines (
    collector_id  TEXT    NOT NULL,
    entity_kind   TEXT    NOT NULL,
    snapshot_json TEXT    NOT NULL,
    captured_at   INTEGER NOT NULL,
    truncated     INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (collector_id, entity_kind)
);

-- The per-key baseline a config diff compares against. keys_json holds key
-- names and already-redacted values, never raw ones.
CREATE TABLE config_snapshots (
    path_hash   TEXT    PRIMARY KEY,
    path        TEXT    NOT NULL,
    keys_json   TEXT    NOT NULL,
    captured_at INTEGER NOT NULL,
    format      TEXT    NOT NULL
);

-- Indexes, each serving a named query.
CREATE INDEX idx_events_timestamp            ON events (timestamp);
CREATE INDEX idx_events_category_timestamp   ON events (category, timestamp);
CREATE INDEX idx_events_entity_timestamp     ON events (entity_id, timestamp);
CREATE INDEX idx_events_event_type_timestamp ON events (event_type, timestamp);
CREATE INDEX idx_events_importance_timestamp ON events (importance, timestamp);
CREATE INDEX idx_relationships_source        ON entity_relationships (source_id);
CREATE INDEX idx_relationships_target        ON entity_relationships (target_id);
CREATE INDEX idx_entities_type_name          ON entities (entity_type, display_name);

-- Defaults. Every collector is on except the two that watch a developer's
-- own tooling, which the user opts into. The files collector is on with an
-- empty watch list, so it records nothing until a folder is chosen.
INSERT INTO settings (key, value, updated_at) VALUES
    ('theme', '"system"', 0),
    ('retention', '{"events_days":30,"snapshots_days":7,"important_days":90}', 0),
    ('monitoring_profile', '"balanced"', 0),
    ('collectors', '{"process":true,"listeners":true,"files":true,"services":true,"applications":true,"updates":true,"drivers":true,"network":true,"startup":true,"eventlog":true,"docker":false,"dev_env":false}', 0),
    ('first_hide_notified', 'false', 0),
    ('onboarding_completed', 'false', 0),
    ('developer_mode', 'false', 0);
