# Public v1 go/no-go packet

**Evidence state: available packet; current decision is NO-GO until every mandatory gate is attached.**

Current decision: **NO-GO**. This packet becomes the single approval surface
only after all required evidence is attached. Publication approval must name the
exact version, artifact hashes, repository tag, hosting URL, wiki projection and
community copy; changing any of those requires a new approval.

| Gate | Required evidence | Current result |
|---|---|---|
| Reproducible release | `scripts/release.ps1` output from a clean tree | Prior dirty rehearsal reached its non-perf gates; no clean/current-source run |
| Rust/frontend verification | complete test, lint, bindings and build logs | [Continuation evidence](../plans/reports/qa-261027-0434-mvp-continuation.md) records frontend typecheck and all 339 tests passing with one worker after the default run failed on worker startup. [Prior Rust verification](../plans/reports/qa-261005-recurrence-rolling-production.md) remains scoped to its source; complete current-source `bun run verify`, bindings parity and rebuilt RC gates remain open |
| Installer | both filenames and SHA-256 for the approved source revision | [Unsigned candidate](../plans/reports/qa-261027-0059-installed-candidate.md) has matching installer copies and verified notices. [Installed UI evidence](../plans/reports/qa-261027-0434-mvp-continuation.md) binds the NSIS executable bytes and records partial acceptance, including a failed workspace investigation. It predates the subsequent source repair; no rebuilt/retested candidate or clean install/UAC evidence |
| Upgrade/uninstall | installed rehearsal with history preserved | Not run |
| Update path | Explicit opt-in control makes no network call without a signed HTTPS feed and embedded public key; manual download remains available. If publishing with the scaffold only, the frozen packet must disclose that limitation for explicit user approval | Scaffold source tests exist; no feed/key configured and no installed update-path check |
| Privacy and diagnostics | purge, byte-scan and export tests | Source tests present; current-candidate save-dialog and installed checks are not run |
| Performance/a11y | P20 corpus, soak, frame and manual records | [Frozen guarded-recurrence matrix](../plans/reports/qa-261005-recurrence-rolling-query-matrix.md) passes all 24 query cells; independent seed target remains missed. [Performance evidence](performance-baseline.md) routes canonical size/full-compaction and routine retention passes, plus the open collector-shaped payload sensitivity, SQL-plan branch parity, 1M upgrade, resource/soak and installed manual requirements |
| Brand/docs/launch | manifest and link/claim checks | Local source checks have recorded passes; current-release installed/publication audit not run |
| Third-party licenses | artifact-scoped inventory and notice texts | [Source packet approved](../plans/reports/approval-261027-0059-notice-packet.md); [candidate notice hashes and installed-file binding](../plans/reports/qa-261027-0434-mvp-continuation.md) verified. Any rebuilt release must bind its own artifacts and notices |
| Support/security | [SUPPORT.md](../SUPPORT.md), [SECURITY.md](../SECURITY.md) | Source review |
| User launch approval | frozen manifest and explicit approval | Pending |

The authorized [public source-preview wiki](deployment-guide.md#public-source-preview-wiki)
is separate from public-v1 approval. No binary release or community launch post
is authorized by this packet. Keep the last known-good installer and checksum when a
later candidate is withdrawn; never roll a database schema backward. The
preserved rehearsal pair is not the frozen public-v1 candidate.
