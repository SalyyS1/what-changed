-- One user-selected known-good checkpoint per registered project.
-- Checkpoints remain global and are not deleted when a project is removed.
ALTER TABLE projects ADD COLUMN baseline_checkpoint_id INTEGER REFERENCES checkpoints(id) ON DELETE SET NULL;
