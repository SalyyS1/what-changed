# Fixed-five initial Clippy failure — 2026-10-05

The first all-target/all-feature release Clippy exits 1 in 2.80 s.
Protocol: [correctness record](qa-261005-fixed-five-seed-grouping.md).
At this point focused 13 and complete bin 17 passed; caller purge 9 and
insertion profile 2 passed with one diagnostic ignored. These results do not
hide the failed lint gate. No repair attempted before supervisory counsel.

```powershell
cargo clippy --release --locked --all-targets --all-features -- -D warnings
```

```text
    Checking what-changed v0.1.0 (D:\Project\.WhatChage\src-tauri)
error: the `Err`-variant returned from this closure is very large
   --> src\bin\seed_history_grouping\seed_grouped_events.rs:118:24
    |
118 |     let value = (|| -> Result<GroupState, GroupFailure> {
    |                        ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ the `Err`-variant is at least 160 bytes
    |
    = help: try reducing the size of `seed_grouped_events::seed_grouped_failure::GroupFailure`, for example by boxing large elements or replacing it with `Box<seed_grouped_events::seed_grouped_failure::GroupFailure>`
    = help: for further information visit https://rust-lang.github.io/rust-clippy/rust-1.98.0/index.html#result_large_err
    = note: `-D clippy::result-large-err` implied by `-D warnings`
    = help: to override `-D warnings` add `#[allow(clippy::result_large_err)]`

error: could not compile `what-changed` (bin "seed_history" test) due to 1 previous error
```

Counsel approves one boxed private payload containing typed primary and optional
rollback causes, preserving the GroupFailure result wrapper/signatures. Only
failure construction allocates; success adds no allocation. Keep typed access,
formatting and both causes; no lint suppression or string-only errors.
Review constructors/accessors and repeat focused failure cases, this Clippy,
then the complementary normal-build Clippy. Unchanged full parity/performance
reruns are unnecessary for this representation-only repair unless a later
check reveals behavior change. No timing/default/acceptance promotion here.
