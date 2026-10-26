-- Checkpoint items use the checkpoint-owned rows already modelled by
-- `snapshots`; these indexes make each side of comparison one bounded indexed
-- read. The explicit manifest distinguishes a successfully captured empty
-- collector from a collector that failed and is missing.
ALTER TABLE checkpoints
    ADD COLUMN captured_collectors TEXT NOT NULL DEFAULT '[]';

CREATE INDEX idx_snapshots_checkpoint
    ON snapshots (checkpoint_id);
CREATE INDEX idx_snapshots_checkpoint_collector
    ON snapshots (checkpoint_id, collector_id);
