# Diagnostics and troubleshooting

**Evidence state: available; export contents are intentionally value-free.**

Diagnostics shows the database state, schema version, storage size, collector
state/cadence and bounded delivery counters. A failed or unavailable collector
does not stop the other collectors; the degraded banner links to Diagnostics.

When reporting a problem, include the app version and the named failed
collector. Export diagnostics instead of attaching history. The export must
not contain event bodies, paths or entity names; the native save dialog asks
where to write the plain-text report.

If the database is corrupt or schema-mismatched, the recovery surface offers
Repair, Export diagnostics or Reset history. Repair keeps a quarantined copy
of the damaged database when rebuilding succeeds. Reset moves the damaged
database and its WAL/SHM companions aside before starting fresh. A healthy
database rejects Repair and Reset through the recovery command; use the
privacy screen's typed `delete` confirmation and one-use challenge for an
intentional history deletion. Recovery never runs automatically.
