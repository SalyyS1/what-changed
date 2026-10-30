# Test Report — P03 Redaction & Privacy Chokepoint

Gate: `cargo fmt --check`, `cargo clippy --all-targets --all-features -D warnings`, `cargo test --all-features`.

| Suite | Result |
|---|---|
| cargo fmt --check | clean (FMT_EXIT=0) |
| cargo clippy -D warnings | clean (CLIPPY_EXIT=0) |
| cargo test (lib unit) | 46 passed |
| capability_allowlist | 1 passed |
| redaction_leak | 7 passed |
| redaction_leak (release) | 7 passed (REL_EXIT=0) |
| doc-tests | 0 (none) |

## What the leak suite proves

12 synthetic secret shapes (`tests/fixtures/secrets.rs`), each replayed through both sinks:

1. `scrub_value(None, value)` — the shape rule must fire with no key to help it, and be attributed to the expected `kind`.
2. A JSON event carrying the value in `before`, `after` and `metadata` → `Sanitizer::sanitize` → serialized; asserted that no 8-character window of the credential survives, that `[REDACTED]` is present, that a redaction was reported, and that `changed: true` and the file path survive.
3. `tracing::error!(payload = %value)` + `tracing::warn!("{key}={value}")` through `RedactingMakeWriter` → the recorded bytes contain no 8-character window of the credential.
4. The value written in two halves across two `write` calls → still no leak (line buffering).

Plus: URI host survives redaction (`db.internal.example`, `cluster0.…mongodb.net`); all 40 benign values round-trip byte-identical through both `scrub_value` and `scrub_line`; corpus sizes pinned at 12 and 40 so a dropped fixture fails the build.

## Bugs found and fixed during verification

1. **Greedy pair value swallowed the next pair.** A single regex for `key=value` matched `config: DB_PASSWORD=hunter2` with key `config`, hiding the inner pair from the rule meant to catch it. Replaced with a hand-rolled separator scan that reads the key immediately left of `:`/`=`, which also rejects `C:\…` (one-letter key) and `14:22` (digit-initial key).
2. **A sensitive key could strand a credential.** `Authorization: Bearer <token>` — the key rule replaced only `Bearer` (unquoted values stop at whitespace so trailing prose survives), leaving the token in the clear because the `Bearer` context the shape rule needs had been removed. Fixed by running shape rules first and having the pair scan absorb an adjacent `[REDACTED]` into one marker. A regression test asserts the token cannot survive.
3. `io::Error::new(ErrorKind::Other, …)` → `io::Error::other(…)`; `s[..n].as_bytes()` → `s.as_bytes()[..n]` (both clippy, deny-level).
4. `into_inner` had no in-crate caller yet (the repository lands later) → explicit `#[allow(dead_code)]` with the reason.

## Performance

Release profile, `src-tauri/examples/redaction_bench.rs`, four ~1 KiB payloads,
2000 rounds each, scaled to one KiB. Full table and reasoning in
`docs/PERFORMANCE.md`.

| Payload | Per KiB |
|---|---|
| config (paths, port, build number, one `DB_PASSWORD`) | 10.8 µs |
| prose, no separator at all — the shape pass alone | 4.0 µs |
| pair scanner's share, by subtraction | 6.7 µs |
| config with a credential — all nine rules run | 14.3 µs |

Budget is 20 µs/KiB; met in every case including the worst.

Measured through an example, not a test, because the release profile's
`panic = "abort"` makes `cargo test --release` build the library twice and link
whichever copy landed last — it compiled on one run and failed on the next with
`serde_json::Value: Scrub is not satisfied`. That is recorded in PERFORMANCE.md
so the next person measuring anything in release does not rediscover it.

Two things the budget cost, both found by measuring rather than guessing:

1. The pair scanner calls `is_sensitive_key` once per `:` or `=`. On a payload of
   Windows paths and build numbers the full rule was allocating a `String` and a
   `Vec<String>` ~96 times per KiB to conclude "not sensitive". A stack-buffer
   stem test answers that without allocating.
2. The generic entropy pattern `[A-Za-z0-9+/]{40,}` has no literal, and one such
   pattern in a `RegexSet` costs the whole set its prefilter — every byte of every
   line got visited by the union automaton. Moving it out of the set and behind a
   byte scan for a 40-character dense run took the common case 13.4 → 10.8 µs and
   the worst case 19.4 → 14.3 µs.

The in-suite ceiling stays deliberately loose (2000 µs/KiB) so it catches a rule
that turns quadratic in any profile without failing on an unoptimised build.

One caution recorded for the next measurement: a run taken immediately after a
three-minute LTO build on a nearly-full disk reported 42 µs/KiB from the same
binary that reported 13.4 minutes later on an idle machine.

## Environment obstacles

- Drive D: filled to 0.13 GB free mid-phase; `cargo` failed with `os error 112` while writing `what_changed_lib.lib`. Freed 8.8 GB with `cargo clean` (regenerable, gitignored build output only).
- The Bash tool was unusable for the whole session (first a wrapper syntax error before any command ran, later silent success with no output and no side effects). All commands ran through Claude Terminal PowerShell tabs, writing to a log file that was then read with the file tools. Terminal tabs were closed by the app twice mid-run and had to be recreated.

## Unresolved questions

- `KEYS`/`HOTKEYS` plural handling: `API_KEYS` and a bare `KEYS` are now redacted (added to the last-word list); `HOTKEYS` and `MONKEYS` are not, because the rule compares whole words. Worth revisiting only if a real config surfaces a sensitive key whose last word is a plural not in the list.
- Whether `PWD` (working directory in shells, password in configs) should stay in the whole-name list. It is currently redacted; the truth table in `plans/reports/research-260903-2300-redaction-key-name-truth-table.md` flags it as deliberate over-redaction.
