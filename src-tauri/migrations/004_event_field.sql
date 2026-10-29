-- A structured config-key change is first-class evidence. Keeping it out of
-- metadata makes it queryable and prevents every consumer from decoding a
-- collector-specific JSON convention.
ALTER TABLE events ADD COLUMN field TEXT;
