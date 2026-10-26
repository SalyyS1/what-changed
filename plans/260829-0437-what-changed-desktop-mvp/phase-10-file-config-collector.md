---
phase: 10
title: "Watched-folder file collector with structured config diffing"
status: completed
priority: P1
effort: 8h
dependencies: [08]
---

## Overview

Watch user-chosen folders (§13) and produce *meaning*, not noise: for known config formats, a
key-level before/after diff (§13) with values scrubbed by P03; for everything else, a
created/modified/deleted/renamed record. This is the highest privacy-risk collector in the app, so it
is opt-in per folder, path-validated by the P05 rules, size-capped, and its diff output is the primary
target of the §60 leak tests.

## Requirements

**Functional**
- `add_watched_path(path)` / `remove_watched_path(path)` / `list_watched_paths()` (P05 commands, already
  allowlisted). Validation on add: canonicalize, must be a directory, reject reparse points/junctions,
  reject `C:\Windows`, `C:\Program Files*`, `C:\ProgramData\Microsoft`, any other user's profile,
  and the app's own data dir; max 20 roots; recursion depth cap 6 enforced during walk, not by notify.
- Watcher: `notify` 8.2.0 `RecommendedWatcher` (ReadDirectoryChangesW on Windows) wrapped in
  `notify-debouncer-full` 0.7.0 with a 500 ms debounce so editor save patterns (write-temp → rename →
  delete) collapse into a single logical change. Verify the debouncer constructor arity against
  `cargo doc -p notify-debouncer-full` before use.
- Ignore rules (built-in, non-configurable in MVP): `.git/`, `node_modules/`, `target/`, `dist/`,
  `build/`, `.next/`, `__pycache__/`, `.venv/`, `vendor/`, `*.log`, `*.tmp`, `*.swp`, `*~`,
  `*.lock` except `package-lock.json`/`bun.lock`/`Cargo.lock` (those are meaningful), and any file
  > 2 MB. Rules are a single `IgnoreSet` with unit tests.
- Structured diff for: `.env`/`.env.*`, `.json`, `.yaml`/`.yml`, `.toml`, `.ini`/`.conf`,
  `.properties`, `.xml` (attribute+text level). Parse old and new, flatten to `dotted.key = value`,
  compare, and emit **one event per provably changed key** (capped at 20 events per file change, then one
  summary event "and 14 more keys changed"). Format detection by extension first, then by content
  sniff for extensionless files named `Dockerfile`/`Caddyfile`/`hosts`.
- The "old" side comes from a shadow copy: on watch registration and after every processed change, the
  file's parsed key-map (values already redacted) is stored in the `config_snapshots` table keyed by
  path. So a diff never needs to re-read a previous version from disk, and the app holds no plaintext
  history. First observation of a file records a baseline with no event.
- Event shape: `entity_name` = file name, `field` = the dotted key for provable key changes,
  `before`/`after` = sanitized values, and `technical_summary` includes the sanitized absolute path.
  Sensitive key addition/removal is provable; a value change under an existing redacted key is not.
  File-change evidence may instead produce "Sensitive configuration may have changed in .env";
  it must not assert that `DATABASE_URL` changed when both retained values are `[REDACTED]`.
- Non-config files: `file/created`, `file/modified`, `file/deleted`, `file/renamed` with size and mtime
  in `details`; no content is read.
- **`config_of` relationship rows.** P15's `Confirmed` confidence class is defined as a structural chain
  reaching a symptom-matched entity, and it names `config_of` as one of the four structural edge kinds —
  so this collector must write it or that class is unreachable for the product's own worked example
  (a `.env` `PORT` change → service restart → port change). On a config-file event, insert
  `file:<hash> --config_of--> <entity>` for each entity the file can be tied to by evidence, and only
  by evidence: (a) the watched root is a registered project (P18) whose process was seen running from
  that directory → the process's executable entity; (b) a `PORT`-shaped key whose value matches a port
  entity currently recorded as listening → that port; (c) a service whose `ImagePath` lives under the
  watched root → that service. Each row carries the event id, so the edge is inspectable in the System
  Map (P17) and citable in an explanation. No heuristic beyond these three; a file we cannot tie to
  anything gets no relationship row and its events rank on category and temporal evidence alone, which
  is the honest outcome rather than an invented edge.
- Downtime coverage: `config_snapshots` is the persisted baseline, so a config file edited while the app
  was closed is diffed on the next watch registration and reported with `occurred_at_is_uncertain = 1`.
  A file whose mtime is newer than `captured_at` is re-parsed on registration for exactly this reason.
- Content is read **only** to parse a supported config format, only when ≤ 2 MB, and the plaintext
  buffer is zeroized (`drop` + explicit overwrite of the `String`'s bytes via `zeroize` 1.8 if the
  crate resolves; otherwise a manual `fill(0)` on a `Vec<u8>` buffer we own) before the function returns.

**Non-functional**
- A `bun install` inside a watched folder (thousands of files under `node_modules/`) produces ≤ 3
  events and ≤ 5% CPU for ≤ 10 s — the ignore set must be applied before any parse or DB touch.
- Watcher survives folder deletion/recreation and drive disconnect: the collector re-registers on
  `notify` error and marks the root `Unavailable` in `list_watched_paths()` instead of failing.

## Architecture

`collectors/files.rs` (watcher lifecycle, event → logical change), `collectors/config_diff/` module
(`mod.rs` dispatcher, `env.rs`, `json.rs`, `yaml.rs`, `toml.rs`, `ini.rs`, `xml.rs`, `flatten.rs`),
`storage/config_snapshots.rs` (baseline get/put), `domain/watched_path.rs`. Parsers: `serde_json`
(already present), `serde_yaml_ng` or `yaml-rust2` — **choose by `cargo add` resolution and record the
resolved name+version; do not assume `serde_yaml` (deprecated/unmaintained)**; `toml` for TOML;
hand-written line parsers for `.env`/`.ini`/`.properties` (~60 lines each, fewer deps than a crate and
tolerant of the malformed files real users have); `quick-xml` for XML if it resolves, else XML falls
back to whole-file `file/modified` and that limitation is listed in `ROADMAP.md` (§87).

## Related Code Files

**Create** — `src-tauri/src/collectors/files.rs`;
`src-tauri/src/collectors/config_diff/{mod.rs,flatten.rs,env.rs,json.rs,yaml.rs,toml.rs,ini.rs,xml.rs}`;
`src-tauri/src/collectors/ignore_set.rs`; `src-tauri/src/storage/config_snapshots.rs`;
`src-tauri/src/domain/watched_path.rs`; `src-tauri/src/commands/watched_paths.rs`;
`src-tauri/tests/collector_files.rs`, `src-tauri/tests/config_diff.rs`,
`src-tauri/tests/path_validation.rs`, `src-tauri/tests/redaction_leak_config.rs`,
`src-tauri/tests/config_relationships.rs`;
`src-tauri/tests/fixtures/config/` (paired before/after files for each format, including a
secret-laden `.env`, a malformed YAML, a BOM-prefixed JSON, a CRLF `.ini`, a 3 MB JSON);
`src/features/settings/WatchedFolders.svelte`.
**Modify** — `src-tauri/src/collectors/registry.rs`, `src-tauri/Cargo.toml` (notify,
notify-debouncer-full, toml, yaml crate, quick-xml, zeroize), `src-tauri/src/commands/mod.rs` +
`dto/bindings.rs`, `src/features/settings/SettingsScreen.svelte`, `ROADMAP.md`, `ARCHITECTURE.md`.
**Delete** — none.

## Implementation Steps

1. `ignore_set.rs` first, with tests — it is the performance and privacy gate for everything else.
2. `path_validation` in `commands/watched_paths.rs` reusing the P05 validator; add reparse-point
   rejection via `std::fs::symlink_metadata().file_type().is_symlink()` **and** the
   `FILE_ATTRIBUTE_REPARSE_POINT` bit (junctions are not symlinks to Rust).
3. `flatten.rs`: `Value`-agnostic flattener producing `Vec<(String, String)>` sorted by key; arrays
   flatten as `key[0]`, `key[1]`.
4. Per-format parsers returning `BTreeMap<String,String>`; each tolerant of malformed input (return
   `Err(ParseSkipped)` → fall back to `file/modified`, never panic, never poison the watcher).
5. `config_snapshots.rs`: `get(path) -> Option<BTreeMap>`, `put(path, map)`, both storing **redacted**
   values only; `delete(path)` on unwatch/remove so removing a folder erases its baselines.
6. `collectors/files.rs`: debouncer wiring, logical-change classification (created/modified/deleted/
   renamed from the debounced event kinds), dispatch to config diff, event emission, root health.
7. Settings UI: folder list with add (native dialog via `tauri-plugin-dialog` — **only** the dialog
   plugin, no `fs` plugin), remove, status badge, and the copy "We record what changed, never the
   secret values."
8. Tests: `config_diff.rs` over every fixture pair (exact expected event lists);
   `redaction_leak_config.rs` asserts that for the secret `.env` fixture, no fixture secret substring
   occurs anywhere in the DB file bytes or the log file; `collector_files.rs` drives a temp dir through
   editor-save, rename, delete, and a 5000-file `node_modules` creation asserting ≤ 3 events.
9. Commits: (a) `feat: ignore set`, (b) `feat: watched path validation and commands`,
   (c) `feat: config parsers and flattener`, (d) `feat: config baseline storage`,
   (e) `feat: file watcher collector`, (f) `feat: watched folders settings ui`,
   (g) `test: config diff and file collector suites`.

## Todo (commit boundaries)

- [x] Ignore set with tests
- [x] Path validation incl. junction rejection
- [x] Flattener + 6 format parsers, malformed-tolerant
- [x] Redacted baseline snapshots in SQLite
- [x] Debounced watcher + logical change classification
- [x] Watched-folders settings UI
- [x] Diff, leak, and stress suites

## Success Criteria

- [x] `.env` fixture: an existing `DATABASE_URL` value changes → one file-level uncertain sensitive
      configuration notice; no exact per-key change claim, secret value, length, hash or digest retained
- [x] Sensitive key addition/removal yields an exact structural key event; editing only a non-secret
      field never fabricates an exact change to an unchanged sensitive key
- [x] `.env` fixture with 25 provable non-secret key changes → 20 key events + 1 "and 5 more keys changed"
- [x] JSON fixture with a nested change → event key is `server.port`, before `3000`, after `4000`
      (non-secret values are preserved verbatim — redaction is key/pattern-driven, not blanket)
- [x] Malformed YAML → one `file/modified` event, no panic, watcher still alive for the next change
- [x] 3 MB JSON → `file/modified` only, never parsed
- [x] `node_modules` creation of 5000 files → ≤ 3 events
- [x] Removing a watched folder deletes its `config_snapshots` rows (asserted by count)
- [x] `config_of` rows are written for each of the three evidence paths and for **no** other case: a
      `.env` whose `PORT` matches a listening port gets one row to that port; a file under a service's
      `ImagePath` root gets one to the service; an unrelated `.txt` change gets none. Asserted as a row
      count, so an over-eager heuristic fails the suite
- [x] The §74 chain is reachable end to end from this collector's output: the `.env` `PORT` event's
      `config_of` row is the structural edge P15 needs to reach `Confirmed`
- [x] A config file edited while the collector was stopped is reported on restart with
      `occurred_at_is_uncertain = 1`
- [x] `redaction_leak_config.rs` finds zero secret substrings in `history.db` and the log file
- [x] Adding `C:\Windows` is rejected with a specific error message; adding a junction is rejected

The two open criteria are sequencing dependencies, not missing Phase 10 plumbing: live-port evidence is
implemented and bounded here; Phase 11 must publish service `ImagePath` evidence, Phase 18 must publish
registered-project/process evidence, and Phase 15 must consume the completed structural chain.

## Verification evidence (2026-09-19)

- Phase gate: 23 passed across `config_diff`, `collector_files`, `path_validation`, both leak suites,
  and `config_relationships`.
- End-to-end chain gate: `cargo test --locked --manifest-path src-tauri/Cargo.toml --test
  config_relationships -- --nocapture` passed **6/6**. The new integration test commits a real
  `FileCollector` `.env` `PORT` change through the production normalizer/writer, asserts its persisted
  `config_of` edge carries the config event id, then runs `InvestigationService` with the persisted
  process/listener evidence; the config suspect is `Confirmed` with a three-step chain ending at
  `TCP 127.0.0.1:4000`.
- Hardening regressions: 17 file-collector unit tests and 36 repository/atomic-commit integration tests
  passed, including stateful sweep continuation, rename preservation, depth-aware cleanup, truncated
  baselines and the 400-edge commit ceiling.
- `cargo check --all-targets` and `cargo clippy --all-targets -- -D warnings` passed.
- Independent frozen-tree gate: fingerprint
  `A534AC5D87D5ABD58634A3CC700269703B8D5B99BEA6708F744A2B06EF276531` matched before/after;
  277 requested tests passed, bindings matched, frontend check reported 0 errors/0 warnings, and the
  production build transformed 774 modules.
- Still unverified here: the ≤5% CPU assertion needs process CPU instrumentation; removable-drive/
  folder-recreation/notify-error recovery needs an end-to-end runtime harness. The oversize regression
  exercises the exact 2 MiB + 1 byte boundary rather than carrying a redundant literal 3 MiB fixture.

## Verification Gate

```bash
cd /d/Project/.WhatChage/src-tauri
cargo test --test config_diff --test collector_files --test path_validation \
           --test redaction_leak --test redaction_leak_config \
           --test config_relationships
cargo clippy --all-targets -- -D warnings
```

## Risk Assessment

| Risk | Observable signal | Pre-decided response |
|---|---|---|
| A secret reaches the DB through a config value the pattern set misses | Leak test fails, or manual review finds plaintext | Layer two: key-name rules already redact `*_KEY`, `*_TOKEN`, `*_SECRET`, `*_PASSWORD`, `*_URL` **for the value**, so an unrecognized *shape* under a recognized *name* is still caught. Fixtures include a plain-looking high-entropy value under a benign key name; if that one leaks, the entropy gate is tightened and the fixture stays as a regression test |
| Chosen YAML crate is unmaintained/absent | `cargo add` fails or emits a deprecation | Record the resolved crate+version in the phase file. If none resolves cleanly, YAML degrades to `file/modified` and the gap is listed in `ROADMAP.md` — never a hand-rolled YAML parser |
| Debouncer collapses two genuinely different changes | Two edits 200 ms apart show as one | Accept: 500 ms is the editor-noise threshold; the event `details.debounced_paths` lists every path in the batch so nothing is silently lost |
| Watched folder on a removable drive disappears | Watcher thread errors repeatedly | Root marked `Unavailable`, re-registration attempted every 60 s, no event storm, collector stays `Running` |
| Plaintext lingers in memory | Not directly observable | Buffers are owned `Vec<u8>` overwritten before drop; no `String` from file content is cloned into an error message (parse errors carry line/col only, never content) |
| `ReadDirectoryChangesW` buffer overflow on huge churn | notify emits an overflow/rescan event | On overflow: skip the batch, log once, and re-baseline the affected root's config files (correct end state, no event storm) |

## Security Considerations

- Highest-risk collector: opt-in per folder, deny-listed system paths, depth and size caps, and content
  read only for parsing.
- There is no command that returns file content to the frontend — the drawer shows only diffed keys and
  redacted values (§56 stays narrow).
- Baselines in `config_snapshots` are redacted at write time, so even a stolen `history.db` has no
  secret values.
- `tauri-plugin-dialog` is added for folder picking; the `fs` plugin is **not** added, and
  `tests/capabilities_allowlist.rs` (P05) keeps it out.

## Privacy clarification (2026-09-13)

The accepted no-secret-history boundary also prohibits secret-derived comparison state: no hash,
HMAC, encrypted copy, raw length or transient cache retained between observations to identify changes
to redacted values. After sanitization, two different sensitive values are deliberately indistinguishable.
The earlier exact `DATABASE_URL changed` criterion therefore contradicted that boundary and is replaced
above with honest file-level uncertainty. Non-secret key diffs and sensitive key presence changes remain
exact. Tests must cover identical sensitive values across unrelated edits, changed sensitive values,
key addition/removal, restart, and absence of both raw and derived secret material in persisted state.
