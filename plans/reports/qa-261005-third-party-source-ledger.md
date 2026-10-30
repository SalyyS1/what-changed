# Pending notice source ledger — 2026-10-05

Outcome: bounded read-only source scouting; authoritative text/provenance
collection precedes manifest review, which precedes frozen-artifact checks.
No edits to legal evidence/approval fields, network fetch, build, test,
installation or publication. Existing gate verification is reused, not rerun.

## Current evidence

`legal/third-party/review-draft.json` remains pending and `publishable=false`:
232 candidates, 399 copied source files, 13 flagged entries / 14 issue labels.
SHA-256: `3703e5681304948b4c449a7d0b2438c502a2d575930f94a6a741535dd128c95d`.
Scouting verifies current Cargo.lock, bun.lock, Cargo.toml and frontend-input
bytes against the four recorded draft hashes; saved Cargo tree/metadata bytes
were not rechecked. [Prior source/gate evidence](qa-261004-retention-native-evidence.md)
already records pending preparation and 96-test native-contract verification.

The actual release manifest `scripts/third-party-notices-manifest.json` remains
pending, reviewer/date null and entries empty. The generator fails at that
guard before dependency/native resolution. Do not copy the draft wholesale or
convert flags into approval fields to make a build pass.

## Unresolved source collection

Eleven candidate records lack full text: alloc-stdlib 0.2.4,
rusqlite_migration 2.6.0, ts-rs 12.0.1, five unic crates 0.9.0,
webview2-com and webview2-com-sys 0.38.2, and frontend plugin-dialog 2.7.3.
Recorded repository routes are discovery inputs, not verified exact-version
license authority. Preserve original text filenames/bytes under the existing
`legal/third-party/<ecosystem>/<name>/<version>/` boundary.

svelte-toolbelt 0.10.6 has copied MIT text/authors but no declared expression.
Its text SHA is `8914f144af4333312a07b30438dc6b75dd8da7a665c3ae9fa751f229eb615e8b`.
option-ext 0.2.0 has MPL text; selected-license/source availability decisions
remain open. Its SHA is `66a3107d5ad6a058aab753eaac2047ccb2ed0e39465dd0fe5844da3e300d5172`.

SQLite native record identifies bundled version 3.53.2 and source ID
`2026-06-03 19:12:13 d6e03d8c777cfa2d35e3b60d8ec3e0187f3e9f99d8e2ee9cac695fd6fcdf1a24`;
`sqlite3/sqlite3.c` is 9,507,037 bytes, SHA
`0a409f1633283fa31a9126b11fbfd64a1991c5d30defad07e5745d4667f5e23d`.
The copied wrapper MIT text does not supply independent SQLite authority.

WebView2Loader native version/license remain null. Three packaged x64 binary
candidate hashes and two build/selection source hashes are evidence, not the
selected executable payload or original SDK terms. Do not infer SDK version
from wrapper 0.38.2. Both native components still lack independent authoritative
source/provenance routes and original native license/NOTICE text evidence.

## Contract and next gate

`reviewed-license-notices.mjs` requires declaration/selected license, obligation
review, explicit NOTICE decision, HTTPS source and hashed local full text;
copyleft choices also require source availability. The native validator binds
independent records to exact contained bytes, selection evidence and Cargo
manifest SHA. A null native version requires explicit provenance review.
HTTPS syntax validation does not fetch authority or establish correspondence;
presence checks do not prove a selected license belongs to its expression.

The generator cannot prove linker retention, installer contents, SDK permission
or final legal approval. Installer verification checks notice/script/hash
surfaces; installed bytes are a separate current-candidate inspection.

Next: source-verified native provenance/terms, then exact-version missing text
collection, keeping review pending. Review selected terms/NOTICE/source routes
after collection; artifact-bound checks require a current release candidate.

Subsequent [native source correspondence](qa-261005-native-notice-source-correspondence.md)
verifies exact official SQLite SHA3 and all three WebView2 SDK candidate bytes,
collecting pending full SDK LICENSE/NOTICE transcriptions. Fresh Cargo metadata
hash differs from this draft despite matching runtime identities/source hashes;
that unresolved snapshot is retained, not silently regenerated or approved.
The subsequent [missing full-text collection](qa-261005-missing-license-source-collection.md)
collects all eleven flagged candidates at verified immutable revisions,
preserving both alternative terms and source/local transcription hashes.
Draft flags/approval fields remain unchanged; collection is not release review.

Initial scouting gaps in exact WebView2 SDK identity/native source authority
are resolved by the later [source correspondence](qa-261005-native-notice-source-correspondence.md);
the [full-text collection](qa-261005-missing-license-source-collection.md) resolves
the missing-text collection step, not preserved draft flags or review approval.
Unresolved questions: raw metadata correspondence, selected terms/NOTICE/source
obligations and final-artifact/installed coverage. No phase/publication gate closes.
