# Checkpoint event-window strict lint output — 2026-10-05

Both terminal exit0, sequential CARGO_BUILD_JOBS=1, cwd src-tauri.
No source edits, ignored workload or acceptance promotion.

```text
cargo clippy --release --locked --all-targets --all-features -- -D warnings
    Checking what-changed v0.1.0 (D:\Project\.WhatChage\src-tauri)
    Finished `release` profile [optimized] target(s) in 2m 20s
```

```text
cargo clippy --release --locked --no-default-features --lib --bins -- -D warnings
    Checking what-changed v0.1.0 (D:\Project\.WhatChage\src-tauri)
    Finished `release` profile [optimized] target(s) in 21.30s
```

Root cargo fmt --all -- --check and scoped git diff --check pass.
All eight frozen source SHA-256 values still match; independent review and
simplifier are CLEAN/no edits. [Ledger](qa-261005-checkpoint-window-capture-ledger.md).

