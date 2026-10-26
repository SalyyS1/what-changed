//! Release-profile measurement of the redaction hot path.
//!
//! This exists as an example rather than as a `#[bench]` or a `cargo test
//! --release` run because the release profile sets `panic = "abort"`, and a test
//! harness needs unwinding. Cargo resolves that by building the library twice —
//! once with abort for normal targets, once with unwind for the test — and both
//! builds write the same filenames into `target/release/deps`. The test then
//! links whichever one landed last, which is how `cargo test --release` compiles
//! on one run and fails on the next with "the trait bound `serde_json::Value:
//! Scrub` is not satisfied": two instances of `serde_json` in one graph. An
//! example is a normal target, so it is built once, with the profile the product
//! actually ships.
//!
//! The suite keeps a loose ceiling of its own (`redaction_leak.rs`) to catch a
//! rule that turns quadratic in any profile. This is the number that goes in
//! `docs/PERFORMANCE.md`.
//!
//! Four payloads, because one figure cannot say where the time goes and
//! `scrub_line` is the only entry point the crate exposes:
//!
//! - `config` is the budget case: realistic config lines, no credential shapes,
//!   many `=` and `:` for the pair scanner to examine.
//! - `prose` has no separator at all, so it isolates the cost of the one
//!   `RegexSet` pass every input pays.
//! - `config` minus `prose` is therefore the pair scanner's share.
//! - `with_secret` adds a credential the shape rules must find, which turns off
//!   the `RegexSet` fast path and runs all nine rules in order — the worst case.

use std::time::{Duration, Instant};

use what_changed_lib::privacy::scrub_line;

/// The budget from the plan, per kibibyte of input.
const BUDGET: Duration = Duration::from_micros(20);

/// Enough rounds that the figure is stable to a tenth of a microsecond.
const ROUNDS: u32 = 2000;

/// Grow `seed` to a kibibyte, then append `tail`.
fn payload(seed: &str, tail: &str) -> String {
    let mut payload = String::new();
    while payload.len() < 1024 {
        payload.push_str(seed);
    }
    payload.push_str(tail);
    payload
}

/// Realistic config: Windows paths with spaces, a port, a four-part build
/// number. Every value here must survive redaction, and each `=` and `:` is a
/// position the pair scanner has to ask about.
fn config() -> String {
    payload(
        "PATH=C:\\Program Files\\nodejs;API_PORT=8080;BUILD=10.0.26200.1234\n",
        "DB_PASSWORD=hunter2\n",
    )
}

/// The same length with nothing for the pair scanner to find.
fn prose() -> String {
    payload(
        "the service restarted and the driver rolled back to its previous build\n",
        "",
    )
}

/// Config with a credential shape in it, so no fast path applies.
fn with_secret() -> String {
    payload(
        "PATH=C:\\Program Files\\nodejs;API_PORT=8080;BUILD=10.0.26200.1234\n",
        "Authorization: Bearer eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiJ0ZXN0In0.c2lnbmF0dXJlCg\n",
    )
}

/// Microseconds per KiB of `input`, measured over `ROUNDS` passes.
fn per_kib(input: &str) -> Duration {
    // Warm the lazily compiled rule set so the first round is not the one that
    // pays for it.
    for _ in 0..64 {
        let _ = scrub_line(input);
    }
    let start = Instant::now();
    for _ in 0..ROUNDS {
        let _ = scrub_line(input);
    }
    let per_round = start.elapsed() / ROUNDS;
    per_round.mul_f64(1024.0 / input.len() as f64)
}

fn main() {
    let config = config();
    let prose = prose();
    let with_secret = with_secret();

    let config_cost = per_kib(&config);
    let prose_cost = per_kib(&prose);
    let secret_cost = per_kib(&with_secret);

    println!("rounds:      {ROUNDS} per payload");
    println!(
        "config:      {config_cost:?} per KiB   ({} bytes, the budget case)",
        config.len()
    );
    println!(
        "prose:       {prose_cost:?} per KiB   ({} bytes, no separator: the shape pass alone)",
        prose.len()
    );
    println!(
        "pair scan:   {:?} per KiB   (config minus prose)",
        config_cost.saturating_sub(prose_cost)
    );
    println!(
        "with secret: {secret_cost:?} per KiB   ({} bytes, all nine rules run)",
        with_secret.len()
    );
    println!("budget:      {BUDGET:?} per KiB");

    if config_cost > BUDGET {
        println!("VERDICT: over budget");
        std::process::exit(1);
    }
    println!("VERDICT: within budget");
}
