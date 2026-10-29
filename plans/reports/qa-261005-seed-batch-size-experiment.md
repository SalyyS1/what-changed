# Bounded seed batch-size experiment — 2026-10-05

Outcome: candidate 5,000 rejected by actual semantic proof; default remains
1,000. No timing experiment or seed acceptance pass.
The [validated insertion attribution](qa-261005-seed-insertion-attribution.md)
measures 195,393.713 ms in commit and 142,507.949 ms in the combined event/entity
loop at 1M. Commit is about 58% of measured repository duration; its SQLite
internals are not separately attributed. The profiler-free seed remains
316,438.748 / 240,000 ms — MISS. Existing observations are context, not the
matched baseline for this experiment.

## Frozen decision and scope

Counsel approves exactly existing batch 1,000 versus one candidate 5,000.
Default and public CLI remain unchanged while testing. Private parameterization
and a test-only runner are allowed; no permanent public experiment flag.
Preserve production writer, sanitizer, insertion SQL, event order, graph
ownership/evidence, checkpoint/investigation recipe and recurrence routing.
Do not change indexes, WAL/NORMAL, 1,000-page autocheckpoint, 8,000 KiB cache,
timeouts, dependencies, query fixture or attribution recorder.

Root source audit: `make_batch` uses global event indexes and cross-batch
relationship ordinals. Known/owned entity sets persist across batches; graph
transactions follow event transactions and must retain that order. Progress
prints at 10k boundaries or the final tail; both selected sizes divide 10k.
FTS insert/delete triggers remain untouched. No proven WAL/FTS cause is claimed.

Worker owns only `src-tauri/src/bin/seed_history.rs` and new private bin-local
test helpers. Root owns execution, formatting, review, protocol/output records
and promotion decisions. Preserve all other dirty changes. No concurrent Cargo
or performance runners; no user AppData/history access or installed app launch.

## Correctness gate before performance

Compare complete persisted event content and order, entities, relationship
evidence, checkpoints/snapshots and investigations at fixed seed/end time.
Normalize only inherently random identifiers/markers or wall-clock fields,
with each exclusion justified against its source. Do not omit event IDs/order,
graph evidence, meaningful timestamps or investigation scores.
Include a nondivisible final batch, repeated entity updates and a real entity
collision. Verify graph ownership and preservation through synthetic purge.

Changing transaction granularity intentionally changes the committed prefix
on a failing seed. Inject failure, prove no partial failed transaction survives,
and prove explicit synthetic purge removes the committed synthetic prefix while
preserving real rows. Close writers before removing only exclusively created,
canonicalized, workspace-owned fixtures; verify normal and unwind cleanup.

Use only selected sizes. Keep allocations batch-bounded to 5,000; account for
additional event/ID/graph input vectors and unchanged persistent ownership and
selection sets. Source bounds alone are not native working-set acceptance.
Production collector batching stays unchanged.

## Predeclared measurement protocol

1. Format and run narrow correctness tests, then affected seeder tests/Clippy.
   Independent reviewer and live-diff conditional simplifier precede timing.
2. Freeze source/artifact hashes. Run one paired 10k smoke in order
   baseline 1,000 → candidate 5,000, sequential fresh fixtures, same explicit
   insertion and coarse timing recorder. Preserve every observation and exit.
3. If correctness/smoke passes, run exactly one fresh canonical 1M per size in
   that same order. Same seed `0xC0DE2026`, end `1800000000000`, 90 days,
   3k entities, 15% relationships, 8% bursts, 40 checkpoints/9,800 snapshots,
   20 investigations. Validate corpus before accepting measured observations.
   Clean each owned fixture before the next. Use pinned release profile, one
   Cargo/job, existing target, owned TEMP/TMP; stage profiling environment unset.
4. Compare total seed time as well as commit/loop time, not commit alone.
   Preserve complete aggregate stdout, errors, hashes and cleanup evidence.
   No outlier exclusions, reruns seeking green, third size or knob sweep.

Promotion requires material total-time improvement, successful full parity and
acceptable bounded-memory/failure tradeoffs, followed by counsel. An ambiguous
or losing result stops this experiment. A faster result still above 240 s does
not close the seed gate. If promoted, run one unchanged profiler-free canonical
seed acceptance with the new default; attribution is not acceptance.

Any unexpected failed verification stops work for counsel before diagnosis or
edits. No phase/checklist/release status changes merely from this experiment.

## Evidence

Source audit identifies a credible successful-output dependency: graph-only
missing endpoints are created after each event batch. At 1,000, an early edge
can create its target before that target's first subject event in a later batch;
at 5,000, that later subject can already exist, preventing the earlier graph
timestamp from being applied. `first_seen` is semantic and cannot be normalized.

Counsel narrows authorization to private parameterization and one deterministic
proof case comparing full persisted events/entities/relationships. The case
must prove an early edge/later subject crosses 1,000 but fits within 5,000.
Default/graph behavior remain unchanged. If any semantic output differs,
preserve the failed evidence and stop before checkpoint parity or performance.
Graph/event interleaving repair requires a separately evaluated design; it is
outside this batch-size-only experiment. If parity passes, prove the branch
was exercised before returning to the broader gates above.

Proof-only implementation is frozen. Public execution explicitly uses 1,000;
candidate 5,000 and its proof module exist only under `cfg(test)`, with no CLI
flag. A separate small fixture test verifies normal/unwind cleanup and sibling
preservation. Full comparisons stream all persisted columns/IDs/order; owned
sets compare literally. No semantic normalization or checkpoint/perf harness.
The ignored negative proof is a characterization: test PASS requires the
suspected `first_seen` mismatch and means candidate REJECTED, not equivalence.

Independent read-only reviewer and live-diff conditional simplifier are clean;
neither edits source or claims execution. Live signals: 7,540 text LOC / 145
tracked files (16 binary) / maximum added 564, exceeding the simplify gate.
Root formats only three owned files; full rustfmt and scoped diff checks pass.
Ordinary seeder tests pass 3/3, zero failures, one explicit proof ignored,
0.13 s after 53.18 s compilation. The single selected negative proof passes
in 1.59 s after 0.63 s cached preparation; PASS means rejection, not parity.
Seed-bin release/devtools Clippy passes with warnings denied in 1.12 s;
all-target/all-feature Clippy including new tests passes in 1.35 s.

Frozen SHA-256, paths relative to `src-tauri/`:

| Source | SHA-256 |
|---|---|
| src/bin/seed_history.rs | 8CBF90234CD533492AFDED1952A765A68270E54DE093B9DD50A76F8BCE5DE6ED |
| src/bin/seed_history_proof/seed_batch_boundary_proof.rs | 6EE5D480C0287D94F19FF46B56320766C13FBEE2CCFB9C0031073BC3E4B2A273 |
| src/bin/seed_history_proof/seed_batch_boundary_fixture.rs | E9CEB334C999B62F940294230606D51B74BE0621C83D2FBE0739012B72182A5A |

Root proof command (same one-job release/owned TEMP environment as attribution):

```powershell
cargo test --release --locked --features devtools --bin seed_history seed_batch_boundary_proof::canonical_cross_boundary_endpoint_rejects_larger_batch_parity -- --ignored --exact --nocapture --test-threads=1
```

The full persisted comparison verifies 10,000 identical events, 1,500 identical
relationship rows and equal complete ownership sets. Of 2,578 event/graph
entities, one differs only in `first_seen`: witness edge index 986, first subject
index 3,186, delta 1,710,891,089 ms. All semantic timestamps remain literal.
Inventory completion was deliberately not run: it cannot repair this semantic
failure or justify broader experiments. [Complete negative-proof stdout](qa-261005-seed-batch-boundary-proof-output.md).

Root verifies all owned proof fixture children absent and seed runners exited.
Proof unit artifact `seed_history-b3d08699438bbe77.exe` SHA-256:
`34FBA1609C6D323E5A7950DE916CDD5928200F4A1DB2A234EE7C1E55D729185B`.
C: free 3,439,996,928 bytes, D: 2,943,680,512 bytes after cleanup. No manual
deletion or user history access. No checkpoint/investigation parity, smoke
timing, 1M size experiment or promotion follows the rejected candidate.
Attribution/query evidence stays frozen to its earlier revision; the proof
does not replace those artifacts. Counsel separately approves proof-first
transaction grouping: five unchanged logical 1,000-event event→graph units
inside one outer transaction, with provisional state discarded on failure.
This is not revival of the rejected event batch. Repository/graph body
extraction must preserve existing callers; instrumentation must distinguish
uncommitted bodies from actual outer commits. Exact API/source audit is next,
no grouping implementation or new acceptance result claimed here.

Unresolved questions: the next parity-safe cause-aligned insertion repair;
independent SQL-plan/resource/native/release obligations.
