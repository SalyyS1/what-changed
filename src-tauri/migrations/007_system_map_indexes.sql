-- Phase 17 walks relationships in both directions and counts distinct
-- neighbours. The original target-only index finds rows but cannot stream
-- their source ids in order; this covering index avoids a temporary sort.
CREATE INDEX idx_relationships_target_source
    ON entity_relationships (target_id, source_id);
