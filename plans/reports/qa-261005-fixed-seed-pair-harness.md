# Fixed seed-grouping pair harness — 2026-10-05

Status: both exact once-per-scale pairs complete; persisted parity and cleanup
pass. Counsel rejects default promotion: 1M still misses 240 seconds and 10k
is slower. Seven ordinary harness tests, 23 ordinary seeder regressions and
complementary strict Clippy pass.
[Accepted correctness](qa-261005-fixed-five-seed-grouping.md) and
[final correctness output](qa-261005-fixed-five-final-verification-output.md)
remain separate from this new measurement harness.

## Approved immutable experiment

Exactly one 10k baseline→fixed-five pair, then only if green one fresh canonical
1M baseline→fixed-five pair. No reruns, tuning, size sweep or excluded observations.
Public/default seeder remains independent 1,000-row logical transactions.
Fixed recipe: 90 days, seed 0xC0DE_2026, end 1,800,000,000,000 ms, 3,000 entities,
15% relationships/8% burst rows, 40 checkpoints/9,800 snapshots/20 investigations.
Collector payload sensitivity false; coarse/insertion attribution true;
investigation stage instrumentation unset. Actual SQLite settings are checked
and pair-compared, including WAL/NORMAL, cache -8000, autocheckpoint 1000 and busy5000.

Identical seed clocks start before actual marker registration, include route
work, bounded ownership clone, real inventory/checkpoint/investigation tail and
Db.flush. Fixture open/settings/validation/digests/report/close/deletion are
excluded explicitly. Legacy repository and grouped body/graph/commit records
keep their distinct meanings; full-seed elapsed is the principal comparison.
One fixed-order pair cannot establish statistical confidence or causal certainty.

Each run uses one exclusive workspace fixture; validate canonical corpus and
stream exact ordered SHA-256 schema/all-column/type/length/float-bit/physical-ID
content for ten reviewed tables, plus sorted ownership. Use only the unchanged
reviewed marker/audit normalization after provenance checks. Close/join/delete
and verify NotFound before constructing the counterpart. Compare row counts and
digests; any mismatch stops. RAII covers normal/error/unwind paths. No two
corpora coexist; no history-sized row collection is retained by the digest.

Before each fixture, native available-to-caller disk check requires 825,786,368
bytes at10k or 2,853,306,368 at1M: 2,048/input +512MiB WAL/temp +256MiB build/
runtime headroom. Historical 1M DB795,389,952/WAL90,594,712 supports a conservative
admission allowance, not a physical growth upper bound. Never relax it to run.
Prepared-input source bounds are 1k versus5k plus fixed3k ordinal/ownership
structures and unchanged history-sized selection temporaries; native peak and
dynamic bytes are unmeasured, not inferred equivalent from corpus equality.

## Ownership and review

Only one private child registration and two helper visibility changes in the
reviewed normalizer, plus six new bin-local measurement/digest/admission modules.
Seeder wrapper, shared library, normalizer bodies, owned fixture, graph/group
control, boxed errors, schema/settings and existing canonical fixture unchanged.
Independent frozen reviewer and conditional simplifier are clean, no edits/check
execution. Live gate 7,664 text LOC/145 tracked files/16 binary/max-added602,
thresholds400/8/200; no disabling config. Scoped Git shortstat unchanged because
bin sources are untracked, so source hashes are the relevant identity proof.

## Root verification so far

Formatting/full rustfmt/scoped diff checks pass. Focused prefix passes 7,
zero failed, two explicit pairs ignored, 17 filtered, 0.01s after1m11s compile.
Strict all-target/all-feature Clippy passes2.08s; normal no-default lib/bins
passes0.77s. Full ordinary seeder command passes23/zero failed/three ignored
in23.62s after1.24s preparation. Never include all ignored selectors because
that would run expensive pairs outside the fixed protocol. Counsel accepts the
verified harness: one exact10k pair, then only if green one exact1M pair;
stop on any parity/cleanup/disk/execution failure and preserve complete output.

Current unit artifact SHA-256:
`5652a5ac0916611a3da8c1c7f609b8eca3f91cd5f41ba67b21a4b3a59b853f87`.
Frozen owner module SHA-256s (under src/bin/seed_history_grouping):

| File | SHA-256 |
|---|---|
| seed_grouped_events.rs | `1b64302f053084be12eab71ba51fa9a9f3df706a955fa92f7a4fd526f69bbfa4` |
| seed_grouped_persisted_parity.rs | `f27b730cb3943f8be639167f834983fa126c2ac7da44e1064d71b0893ca60d7a` |
| seed_grouped_content_digest.rs | `e28b078d9fa9fbd73347e12b1fab10e16f7cfc7a751b37a144e68de5c281e692` |
| seed_grouped_digest_tests.rs | `3abe0393aa6d19bac053c7784dcae5a0835bc0f378a6ab685a74dce2c89aa4ff` |
| seed_grouped_admission.rs | `c2494020394ec5bd0fe576603799bdbb975574308b377a3c5a5a56afc8bb84b6` |
| seed_grouped_corpus_validation.rs | `1ebe9849ebf4549354c7431d12c937eb819eb2b7d8c6f904005cb2b536ae7f9c` |
| seed_grouped_measurement_run.rs | `5f3dd03750e05599970cb9250026bed5a1498ddb99e080e77ba51679431fdc4f` |
| seed_grouped_measurement.rs | `afc72dce6c84828c0d15d4948fb8e7494cf26bf8985ce2d20efc3b6ce9351d64` |

Root rehashes after reviewers and before pair execution; no default promotion,
240-second acceptance or memory/native/release gate claim.
The exact 10k pair passes complete normalized content/count/ownership parity
and verified sequential fixture removal. Baseline1,109.966ms versus grouped
1,257.987ms is a slower observation, not an improvement. All output is retained
in [the 10k pair receipt](qa-261005-fixed-seed-10k-pair-output.md).
The single exact1M pair passes complete normalized content/count/ownership
parity: baseline369,085.288ms versus grouped332,180.967ms (~10.00% less).
Grouped still misses240,000ms by92,180.967ms; neither diagnostic run replaces
the profiler-free canonical316,438.748ms acceptance MISS. Baseline commit
195,463.706ms versus grouped128,794.004ms is not attribution of SQLite-internal
checkpoint/I/O work. One fixed-order pair cannot establish general speedup.
The command passes one test/zero failures in736.74s after1.60s preparation.
Complete stdout is partitioned without omissions into
[baseline output](qa-261005-fixed-seed-1m-baseline-output.md) and
[fixed-five output](qa-261005-fixed-seed-1m-fixed-five-output.md).

Post-run checks confirm all eight owner hashes above unchanged, frozen unit
artifact/seeder wrapper/shared insertion owner/unchanged PerfFixture identities,
absent owned fixture prefixes and no owned Cargo/test/seeder runners. Both
1M fixtures were closed/joined/deleted sequentially before counterpart creation.
Counsel rejects promotion and any additional size/pair tuning; private cfg(test)
grouping and default1,000 route stay unchanged. Next: approved actual-bound
[page capture slice](qa-261005-page-plan-capture-ledger.md), with a separate
read-only source scout for commit-internal evidence before any further seed fix.
No phase checkbox, native/resource/release verdict or public-v1 NO-GO changes.
Unresolved questions: native memory cost, commit-internal work and remaining
independent full-plan obligations.
