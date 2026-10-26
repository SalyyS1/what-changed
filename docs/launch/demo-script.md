# 60–90 second demo script

**Evidence state: fixture verified in source tests; tagged-release-candidate
walkthrough and publication approval remain open.** The `demo` profile writes
eight sanitized `synthetic` events and four persisted edges. The integration
test confirms strict chronology, canonical entity IDs, secret redaction, edge
evidence, and at least one ranked investigation result with a `.env`–port
chain. It does not prove the final UI, installer, or recording is ready.

## Prepare the recording safely

1. Use a clean, isolated Windows VM or dedicated Windows user account with no
   personal What Changed history. Verify that account's app-data folder has
   no prior database; **do not delete or overwrite an existing user's data**.
2. Run the debug/devtools build, enable Developer mode in Settings, then choose
   **Linked demo story (8 events)** in the Event simulator and seed it once.
   Relations are mandatory for this profile, regardless of the generic switch.
   The fixture runs through the normal sanitizer and writer; it reads no real
   `.env` file, service, process, or socket. Seeding events and graph edges is
   not one atomic transaction; if this step errors or the expected eight rows
   and four edges are absent, discard this isolated QA profile and recreate it
   from clean state. Do not retry into a partial database or touch user data.
3. Close the devtools build. Open the **ordinary tagged release candidate** in
   that same isolated account so it reads the pre-seeded local database. The
   shipped release handler has no simulator command. Confirm all eight rows,
   the source label `Synthetic event generator` in Details, and the ranked
   evidence chain in the actual RC before filming. If the build or UI fails that check, stop;
   do not substitute devtools footage and call it the RC.
4. Put a persistent, plainly visible **Synthetic demonstration** disclosure
   on every published frame and screenshot. Timeline rows do not carry a
   synthetic badge by themselves; the Details source label alone is not an
   always-visible disclosure. Include release version and commit in the media
   metadata/caption, not as a false in-app status.

The observed order is: port 3000 opened (baseline), `.env` `PORT` changed from
3000 to 8080, service and process stopped, port 3000 closed, service and
process started, then port 8080 opened. `ConfigOf` edges from `.env` to the
service and process are attached to the config observation. `ListensOn` edges
to ports 3000 and 8080 are attached to their respective *opened* observations;
the older 3000 edge is historical, not proof that it is still listening. The
service is a separate branch from `.env`, not a service→process edge. Every
relationship is fictional fixture evidence, not a causal conclusion about a
real machine.

1. **0–10 s — calm start.** Show Home and say: “This is local history, not a
   live everything monitor.”
2. **10–28 s — observed changes.** Show the pre-seeded `.env` `PORT` change,
   the service/process transition and the old/new listener observations in
   Timeline. Keep the synthetic disclosure visible; do not click Seed in the
   ordinary RC.
3. **28–48 s — investigate.** Enter “port 3000 stopped responding”; open the
   ranked result whose evidence chain shows the `.env` and port observations.
   Show the recorded relationship; call it “related evidence” without asserting
   a proven explanation for the outage. Do not imply the historical 3000
   listening edge is the current socket state.
4. **48–63 s — trust.** Open `.env` Details. Show visible `PORT` values beside
   `[REDACTED]` token values and the `Synthetic event generator` source label.
   A checkpoint comparison is a separate shot, only if its own fixture and RC UI were
   independently verified.
5. **63–78 s — limits.** Show Diagnostics and describe only the degraded state
   actually present in that isolated account. Do not script Docker-unavailable
   as an observed fact unless the RC displays it.
6. **78–90 s — close.** Show the unsigned install note and the documentation
   link. No real username, path, secret, notification or network graph may be
   visible. Before publication, review the final capture for real names,
   paths, secrets, notifications and network graphs; the source tests do not
   perform that media review.
