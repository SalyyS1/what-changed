---
phase: 03
title: "Redaction & Privacy Chokepoint"
status: completed
priority: P1
effort: 4h
dependencies: [01]
---

## Overview

Build the secret-redaction engine and make it structurally unavoidable **before** any persistence
or logging code exists (§34: "redaction runs before persistence and before logging"). Two
enforcement mechanisms: a `Sanitized<T>` newtype whose only constructor lives in
`privacy::sanitize`, and a redacting `MakeWriter` through which every byte of every log line passes.

## Requirements

**Functional**
- Key-name redaction (§13/§34): normalized key (lowercase, strip `_ - . space`) matched against
  `password, passwd, pwd, secret, token, apikey, authorization, cookie, session, privatekey,
  clientsecret, accesskey, refreshtoken, bearer, credential, passphrase`, plus suffix rules
  `*key`, `*token`, `*secret`.
- Value-pattern redaction: `sk-[A-Za-z0-9_-]{16,}`, `Bearer\s+\S{12,}`, JWT
  `eyJ[A-Za-z0-9_-]{8,}\.[A-Za-z0-9_-]{8,}\.[A-Za-z0-9_-]{8,}`, `AKIA|ASIA[0-9A-Z]{16}`,
  `-----BEGIN [A-Z ]*PRIVATE KEY-----`, `gh[pousr]_[A-Za-z0-9]{20,}`, `xox[baprs]-\S+`,
  URI userinfo `(?i)\b[a-z][a-z0-9+.-]*://[^\s/@:]+:[^\s/@]+@` (redacts credentials, keeps host —
  covers `postgres://`, `mysql://`, `mongodb+srv://`, `redis://`, `amqp://`), and a gated
  high-entropy rule for `[A-Za-z0-9+/]{40,}={0,2}` that **excludes pure hex and pure digits** so
  file hashes, versions, ports and PIDs survive.
- Redacted output is the literal `[REDACTED]`. For a config diff, `changed: bool` is computed in
  memory from the pre-redaction values and the values are then dropped — never a digest, never a
  salt, never a prefix. This is what makes §13's `OPENAI_API_KEY [REDACTED] → [REDACTED]` honest.
- `Scrub` trait implemented for the event payload types' string/JSON fields; `Sanitizer::sanitize<T:
  Scrub>(T) -> Sanitized<T>`; `Sanitized<T>` exposes `into_inner()` only to `storage`.
- `scrub_line(&str) -> Cow<str>` applied by `RedactingMakeWriter` to stdout **and** the rolling file.
- Tracing setup: `EnvFilter` with a runtime-reloadable handle (§4 configurable verbosity), rolling
  daily file in the app log dir via `tracing-appender`, JSON-ish structured fields, console layer
  in dev only.
**Non-functional**
- Redaction cost < 20 µs per 1 KB payload (benchmarked); regexes compiled once via
  `std::sync::LazyLock` (no `once_cell` dependency).
- False-positive discipline: a fixture of 40 benign values (paths, versions, GUIDs, sha256 hashes,
  ports, KB numbers, driver versions, container ids) must pass through **unchanged**.

## Architecture

```
collector → NormalizedEvent ──► Sanitizer::sanitize ──► Sanitized<NormalizedEvent> ──► EventRepository
tracing macro → subscriber → RedactingMakeWriter::scrub_line → {stdout, rolling file}
```
`Sanitized<T>(T)` has a private field; the module exposes no `new`, no `From`, no `Deref`. The
repository signature `fn insert_batch(&self, events: &[Sanitized<NormalizedEvent>])` therefore makes
an unsanitized write a compile error. `diagnostics` (P19) and `commands` reuse `scrub_line` — one
implementation, three consumers (DRY).

## Related Code Files

**Create** — `src-tauri/src/privacy/mod.rs`, `src-tauri/src/privacy/sanitize.rs` (newtype +
`Sanitizer`), `src-tauri/src/privacy/scrub.rs` (`scrub_line`, `scrub_value`),
`src-tauri/src/privacy/patterns.rs`, `src-tauri/src/privacy/keys.rs`,
`src-tauri/src/privacy/entropy.rs`, `src-tauri/src/observability/mod.rs`,
`src-tauri/src/observability/tracing_setup.rs`,
`src-tauri/src/observability/redacting_writer.rs`, `src-tauri/src/error.rs` (typed
`AppError` via `thiserror`), `src-tauri/tests/redaction_leak.rs`,
`src-tauri/tests/fixtures/secrets.rs`, `src-tauri/tests/fixtures/benign_values.rs`.
**Modify** — `src-tauri/src/lib.rs` (init tracing first, before anything else),
`src-tauri/Cargo.toml` (`tracing`, `tracing-subscriber` 0.3.23 + `env-filter`,
`tracing-appender` 0.2.5, `regex`), `SECURITY.md`, `PRIVACY.md`.
**Delete** — none.

## Implementation Steps

1. `patterns.rs`: one `LazyLock<Vec<(Kind, Regex)>>`; `Kind` is an enum used for the redaction
   reason recorded in metadata (`{"redacted": {"field": "OPENAI_API_KEY", "kind": "api_key"}}`).
2. `keys.rs`: `normalize_key()` + `is_sensitive_key()`; exhaustive unit tests including
   `API_KEY`, `api-key`, `Api.Key`, `X_AUTH_TOKEN`, `db_password`, `SESSION_ID`, plus negatives
   (`API_PORT`, `KEYBOARD_LAYOUT`, `TOKENIZER_PATH`, `SESSION_TIMEOUT_MS` → must NOT redact the
   value but *may* redact on suffix rules; assert the exact intended behaviour per name and
   document any deliberate over-redaction).
3. `entropy.rs`: Shannon entropy over the candidate; require ≥ 3.6 bits/char AND mixed case or
   symbols AND not-all-hex before the generic base64 rule fires.
4. `scrub.rs`: `scrub_value(key: Option<&str>, value: &str)` (key rule first, then pattern rules)
   and `scrub_line(line: &str)` (pattern rules + `key=value` / `"key": "value"` detection).
5. `sanitize.rs`: `Scrub` trait (`fn scrub_in_place(&mut self, s: &Sanitizer)`), impls for
   `String`, `Option<String>`, `serde_json::Value` (recursive, key-aware), and a derive-free manual
   impl for the P04 event struct (added in P04 as a one-line impl).
6. `redacting_writer.rs`: `RedactingMakeWriter<W>` implementing `MakeWriter`, buffering a line and
   calling `scrub_line` before `write_all`. Wire in `tracing_setup.rs` for both sinks.
7. `tracing_setup.rs`: `init(log_dir: &Path, level: Level) -> ReloadHandle`; daily rolling file
   `logs/what-changed.log`, 7-file retention, `EnvFilter` default `info,what_changed=debug` in dev.
8. `redaction_leak.rs` (§60, mandatory): for each of the 12 secret fixtures — (a) sanitize an event
   whose `before`/`after`/`metadata` carry it and assert the serialized JSON contains `[REDACTED]`
   and does **not** contain any 8+ char substring of the secret; (b) `tracing::error!(payload = %s)`
   it, flush, read the log file back, and assert the same. Then the benign fixture must round-trip
   byte-identical.
9. Write the "What we never collect" section of `PRIVACY.md` and the secret-handling section of
   `SECURITY.md` from the implemented rules (not aspirationally).
10. Commits: (a) `feat: secret redaction engine`, (b) `feat: redaction-wrapped tracing`,
    (c) `test: secret leak suite`.

## Todo (commit boundaries)

- [x] Key + pattern + entropy rules with unit tests
- [x] `Sanitized<T>` newtype with single constructor
- [x] Redacting writer + tracing init + rolling files
- [x] Leak suite (DB-bound assertions added in P04) + benign no-op suite
- [x] PRIVACY.md / SECURITY.md sections written from code

## Success Criteria

- [x] `cargo test --manifest-path src-tauri/Cargo.toml redaction` passes with 12 secret shapes × 2 sinks
- [x] Attempting to construct `Sanitized` outside `privacy::sanitize` fails to compile (a
      `tests/compile_fail/` note documents the attempt; the private field is the guarantee)
- [x] All 40 benign values survive unchanged
- [x] Log file after the suite contains zero fixture secret substrings (grep-asserted in the test)
- [x] Redaction bench under 20 µs / KB recorded in `docs/PERFORMANCE.md`

Evidence, all from `plans/reports/tester-260904-0010-p03-redaction-privacy-chokepoint.md`:
49 library unit tests, 1 capability test, 8 leak tests; `cargo fmt --check` and
`cargo clippy --all-targets --all-features -D warnings` clean. Each of the 12
secret shapes is replayed through four artifacts — `scrub_value` with no key to
help it, a serialized event carrying it in `before`/`after`/`metadata`, the log
writer, and the rolling file that `observability::init` actually installs — and
each is searched for any 8-character run of the original. The fourth was added
because the other three build their own subscriber and so prove the writer rather
than the wiring. Redaction costs 10.8–13.8 µs per KiB on a config payload and
14.3–18.0 µs when a credential is present and all nine rules run.

The `tests/compile_fail/` note lists seven ways to obtain a `Sanitized<T>` without
the sanitizer — the tuple constructor, a `new`, a `From`, a `Default`, a
`Deserialize`, the field, and `into_inner` from outside the crate — with the error
each currently produces. It is not compiled (cargo builds only the top level of
`tests/`), because proving it with `trybuild` would add a proc-macro dependency to
assert what the type system already refuses.

## Verification Gate

```bash
cd /d/Project/.WhatChage
bun run rs:fmt && bun run rs:lint
cargo test --manifest-path src-tauri/Cargo.toml redaction -- --nocapture
cargo test --manifest-path src-tauri/Cargo.toml --test redaction_leak
```

## Risk Assessment

| Risk | Observable signal | Pre-decided response |
|---|---|---|
| Over-redaction destroys useful evidence | Benign fixture fails, or a config diff shows `[REDACTED]` for `API_PORT` | Tighten the entropy gate and the suffix rules; the benign suite is the regression guard and must never be weakened to make a pattern pass |
| Under-redaction (a new secret shape) | Leak suite fails / manual review finds a raw token | Add the shape to `patterns.rs` with a test in the same commit; the leak suite is append-only |
| A future collector bypasses the chokepoint by writing SQL directly | Architecture test in P04 detects `INSERT INTO` outside the repository | Fail the build; the repository is the only writer |
| Log line splitting hides a secret across two writes | Secret appears intact in the file | `RedactingMakeWriter` buffers until `\n` before scrubbing; test asserts a split-write case |
## Security Considerations

- The engine is the §82 mitigation for "secret leakage" and part of the mitigation for "malicious
  config file" (a config crafted to smuggle a token into metadata is scrubbed on the same path).
- Redaction happens in-process before the value can reach SQLite, a log file, a Tauri event, or a
  diagnostics export — all four consumers sit downstream of `Sanitizer`/`scrub_line`.
- The leak suite is a *security control*, not a nicety: it may never be `#[ignore]`d.
- No secret-derived material (hash, prefix, length-preserving mask) is stored; only a `changed` flag.
