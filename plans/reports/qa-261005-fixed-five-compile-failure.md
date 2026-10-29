# Fixed-five initial compilation failure — 2026-10-05

First focused command exits 1 during compilation; no tests ran.
Protocol: [fixed-five correctness](qa-261005-fixed-five-seed-grouping.md).
Seeder frozen SHA: `7433E762232181411AB9C5258A46BC26050B7F32695958963C0308C92B6DCC08`.
Other initial source hashes remain in root's freeze record. No repair attempted
before supervisor received the exact command and complete output below.

```powershell
cargo test --release --locked --features devtools --bin seed_history seed_grouped_events:: -- --test-threads=1
```

One root-owned Cargo/job, existing target, owned `.perf-temp` TEMP/TMP and
unset investigation stage environment. Initial command elapsed 7.48 s.

```text
   Compiling what-changed v0.1.0 (D:\Project\.WhatChage\src-tauri)
error[E0277]: the trait bound `usize: FromSql` is not satisfied
   --> src\bin\seed_history_grouping\seed_grouped_persisted_parity.rs:211:13
    |
211 |         row.get(0)
    |             ^^^ the trait `FromSql` is not implemented for `usize`
    |
    = help: the following other types implement trait `FromSql`:
              f32
              f64
              i16
              i32
              i64
              i8
              isize
              u16
            and 2 others
note: required by a bound in `Row::<'_>::get`
   --> C:\Users\Salyyy\.cargo\registry\src\index.crates.io-1949cf8c6b5b557f\rusqlite-0.40.2\src\row.rs:285:32
    |
285 |     pub fn get<I: RowIndex, T: FromSql>(&self, idx: I) -> Result<T> {
    |                                ^^^^^^^ required by this bound in `Row::<'_>::get`

error[E0283]: type annotations needed
  --> src\bin\seed_history_grouping\seed_grouped_transaction.rs:47:42
   |
47 |             Err(GroupFailure::from(error.into()))
   |                 ------------             ^^^^
   |                 |
   |                 type must be known at this point
   |
note: multiple `impl`s satisfying `GroupFailure: From<_>` found
  --> src\bin\seed_history_grouping\seed_grouped_failure.rs:11:1
   |
11 | impl From<AppError> for GroupFailure {
   | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
   = note: and another `impl` found in the `core` crate: `impl<T> From<T> for T;`
help: try using a fully qualified path to specify the expected types
   |
47 -             Err(GroupFailure::from(error.into()))
47 +             Err(GroupFailure::from(<DbError as Into<T>>::into(error)))
   |

Some errors have detailed explanations: E0277, E0283.
For more information about an error, try `rustc --explain E0277`.
error: could not compile `what-changed` (bin "seed_history" test) due to 2 previous errors
```

Counsel approves only explicit i64→usize checked conversion in the private
COUNT helper and explicit DbError→AppError→GroupFailure conversion. Negative
or overflowing counts fail; no casts/zero fallback. Primary/cleanup handling
stays unchanged. Root then reformats and repeats the exact focused command.
No test, latency, failure-contract, default or acceptance gate passes here.
