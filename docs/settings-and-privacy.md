# Settings and privacy

**Evidence state: available; secure erasure is best effort on unencrypted storage.**

History is kept under the per-user application-data directory resolved by the
OS. The app has no account, telemetry, crash-reporting upload or runtime HTTP
client. Watched folders are opt-in and can be removed; their configuration
snapshots and descendant baselines are removed with them.

On a fresh install, the persisted collector choices are only preferences: the
runtime keeps every collector disabled until the welcome tour is finished or
skipped. Completion applies the chosen sources before it creates the first
automatic `First day` checkpoint. If applying a source, creating that
checkpoint, or saving completion fails, the already-started workers are rolled
back and the tour remains open. Reopening the tour keeps existing history but
stops the selected workers until the tour is finished again.

Sensitive values are redacted before comparison and persistence. Exact
presence changes can be reported, but two different values that both sanitize
to `[REDACTED]` are intentionally indistinguishable across a restart.

Deletion controls explain their scope. Category and folder purges remove only
the requested history, including matching descendant configuration baselines,
then rebuild search indexes and reclaim database pages where supported. Delete
everything requires typing `delete` and a short-lived, one-use backend
confirmation. It replaces the database with a freshly migrated file, clears
history, logs and app-owned migration backups, and preserves only the reviewed
theme and visualization preferences; onboarding is required again before
collection resumes. If staging or integrity verification fails, the old
database is retained and the runtime choices are restored for a safe retry.
A normal unencrypted filesystem cannot provide a cryptographic erasure
guarantee.

Recovery is a separate safety boundary. Repair and reset are accepted through
the recovery surface only when the database is corrupt or schema-mismatched;
the healthy database rejects those destructive recovery actions. Repair keeps a
quarantined damaged copy when it can rebuild a file. Reset moves the damaged
database and its WAL/SHM companions aside and starts an empty one. Export
diagnostics contains aggregate health information without history. These are
user-initiated actions; no automatic reset is performed.
