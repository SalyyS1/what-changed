-- Phase 15 reads and retains investigations independently from event retention.
-- These indexes keep window ranking, history, cascade deletes and the fixed
-- 90-day investigation purge bounded without changing any previously applied
-- migration.
CREATE INDEX idx_events_analysis_window
    ON events (timestamp, importance, id);
CREATE INDEX idx_events_recurrence
    ON events (entity_id, category, event_type, timestamp);
CREATE INDEX idx_relationships_recorded_at
    ON entity_relationships (recorded_at, source_id, target_id);
CREATE INDEX idx_relationships_event_id
    ON entity_relationships (event_id);
CREATE INDEX idx_investigations_created_at
    ON investigations (created_at, id);
CREATE INDEX idx_analysis_sessions_investigation
    ON analysis_sessions (investigation_id, id);
CREATE INDEX idx_analysis_results_session_rank
    ON analysis_results (session_id, rank, id);
