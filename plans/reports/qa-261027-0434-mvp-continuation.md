# MVP continuation evidence — 2026-10-27

Status: partial installed-candidate verification; public v1 remains **NO-GO**.
No phase acceptance checkbox is closed by this report. The accepted
[MVP plan](../260829-0437-what-changed-desktop-mvp/plan.md) owns phase status.

## Candidate identity

The unsigned 0.1.0 candidate was built from the dirty source snapshot recorded in
[installer candidate evidence](qa-261027-0059-installed-candidate.md). It predates
the subsequent workspace source repair. It is not evidence for a rebuilt
current-source candidate.

| Artifact | SHA-256 |
|---|---|
| Raw and stable-name installer copies | `379a5739e754af4d2189077592c65a3b0977368821756f91fde094c9d1154da3` |
| Generic build executable | `7a05b560f6a9b5e6df4158ab2211a851c67014c0eb07e4f951fa15c4c831e0c5` |
| Installed NSIS executable | `ad25eadaa8aba5dfd0c40ce6332f52c33949b9ebee45b50de4751f68c1582ce7` |
| Installed third-party notices | `aea1d355d6864858959b2a9e4c8ee628acd56718d1e6a8b2620961f344f8c6a5` |

The generic and installed executable hashes differ because the Tauri NSIS
bundler replaces the bundle-type marker. The installed executable matched the
derived NSIS payload byte for byte; only the three expected marker bytes differ
from the generic build executable. The historical generic hash remains valid
for that artifact. Private provenance and focused corruption/marker checks:
`.qa/installer-artifacts-nsis-payload-20261027.json`.

## Installed native checks

This was verification of an existing installation with isolated app and WebView
data. No installer wizard was invoked; clean-account install, UAC behavior,
version-over-version upgrade and uninstall remain unverified.

| Check | Measured result and limit |
|---|---|
| Launch and onboarding | Native window visible in 1.33 seconds; onboarding completion persisted. Does not establish the complete fresh-install worksheet |
| Watched workspace | Native folder picker selected a disposable fixture; working baseline checkpoint 3 persisted |
| File collection and Timeline | Synthetic `.env` changes produced visible events 617/618. A file event's persisted `related_entity_ids` was empty |
| Workspace investigation | Symptom and checkpoint-window investigation persisted, but reported `no_recorded_evidence_in_workspace`, zero analyzed events and no suspects despite those recorded changes. The workspace product loop failed; investigation/ranking acceptance is not passed |
| Navigation | Seven primary routes rendered at 1280×729 CSS pixels and 150% DPI without overflow or recorded errors. Does not establish keyboard-only use, minimum viewport, high contrast, Narrator or full accessibility |
| Clipboard | API success observed; a later physical Windows observation with explicit UTF-8 decoding matched the exact Markdown pack. Original clipboard restoration was not established. No clipboard contents retained in this report |

Private evidence: `.qa/native-installed-ui-20261027-ui2/`, including
`onboarding-completion-followup.json`, `timeline.json`, `investigate.json`,
`event-membership-followup.json` and `navigation.json`. These files and personal
profile data are not public projection inputs. A stage's automation `passed`
flag does not override the failed product result recorded above.

Clipboard harness follow-up found that Windows PowerShell 5.1 read the UTF-8
JSON expectation with its default Windows-1252 decoding. Explicit UTF-8 decoding
produced a different expected string. The earlier exact-content comparisons are
therefore not valid product-failure evidence and are not promoted to passes.
A later read-only physical Windows STA observer with explicit UTF-8 decoding
matched the complete expected pack. That establishes the later clipboard
content, without changing the failed historical test records or proving its
content at the first copy instant. Original clipboard restoration remains
unproven. Private proof: `.qa/clipboard-utf8-physical-followup-20261027.json`.

## Cleanup and restoration

The native controller reached terminal `complete` with no failure or cleanup
notes. QA exited normally with code 0. Profile recovery, startup Run entry and
metadata restoration were verified; the contents journal reached `Restored`.
Recovery is not required, and the original monitoring app was intentionally
restarted and retained. Clipboard restoration is not established by those
profile checks.

Private owners: `.qa/native-installed-acceptance-session-20261027-ui2.json` and
`.qa/native-profile-contents-20261027-ui2.json`.

## Frontend source verification

- `bun run check`: zero errors and warnings.
- Default test run failed: 54 files/279 tests passed, with 11 worker-startup
  timeout errors. This is an observed infrastructure failure, not a full pass.
- Unchanged full suite with `--maxWorkers=1`: 65 files/339 tests passed in
  455.49 seconds. The successful rerun does not establish all release gates.

Private logs: `.qa/resume-261027-0434-frontend-check.log`,
`.qa/resume-261027-0434-frontend-tests.log` and
`.qa/resume-261027-0434-frontend-single-worker.log`.

## Workspace source repair

The scoped query now includes recorded file paths below the exact registered
workspace root, alongside existing direct and related entity associations.
Membership uses the current registration for new investigations; saved results
remain unchanged. Only file-collector events with a string metadata path qualify.
Canonical path equality and separator boundaries exclude sibling-prefix paths
and treat `%` and `_` literally. No causal relationship is created.

Focused Rust verification passed 17 tests: six new workspace regressions, seven
investigation-service tests and four Change Map tests. The regressions include
historical `.env` keys without running process evidence, snapshot immutability,
unknown roots and more than 5,000 unrelated changes. Debug query observations
were 133.96 ms for 5,102 stored/two matching rows and 399.28 ms for 10,101
stored/5,001 matching rows; these are not production-scale acceptance results.

All-features library verification passed 571 tests with three explicit ignores.
Strict Clippy passed for the library and three affected integration targets.
Formatting, bindings parity and the resolved Windows no-network gate passed.
Independent review found no correctness/security defect; 100k/1M workspace
query performance remains unverified. Private logs:
`.qa/resume-261027-workspace-regression.log`,
`.qa/resume-261027-workspace-clippy.log`,
`.qa/resume-261027-library-tests.log` and
`.qa/resume-261027-bindings-check.log`.

Frontend tests now use one worker by default. The ordinary `bun run test`
passed all 65 files/339 tests in 762.10 seconds; no tests or assertions were
removed. Final frontend typecheck again reports zero errors/warnings. Logs:
`.qa/resume-261027-default-single-worker.log` and
`.qa/resume-261027-final-frontend-check.log`.

## Public documentation route

Read-only GitHub metadata verified `SalyyS1/what-changed` is public and Pages uses
a workflow with URL [What Changed source-preview wiki](https://salyys1.github.io/what-changed/).
The source preview was authorized by the user. This check neither published a
binary nor established hosted availability or a current deployment revision.
The operational lookup route is owned by the
[deployment guide](../../docs/deployment-guide.md#public-source-preview-wiki).

## Remaining acceptance

Rebuild and repeat the native product loop against the repaired artifact identity.
Installed upgrade/uninstall, update path, privacy/diagnostics, full accessibility,
resource/network observation, performance/soak, clean-tree release and explicit
frozen-manifest publication approval remain open. See the
[worksheet](../../docs/acceptance-walkthrough.md) and
[NO-GO packet](../../docs/public-v1-go-no-go.md).

Documentation validation: `node scripts/check-docs.mjs` passed (33 Markdown
files and relative links); scoped `git diff --check` passed.

Unresolved questions: none; remaining gates require evidence, not a new scope decision.
