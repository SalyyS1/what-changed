# COMMIT PROFILE dependency-feature isolation — 2026-10-05

Status: normal Windows resolved rusqlite feature tree unchanged, byte-for-byte
stdout equality before/after devtools-only trace forwarding. Both commands
exit0; actual source compilation/lint and lifecycle gates remain pending.
[Accepted protocol](qa-261005-commit-profile-source-ledger.md).

Command: `cargo tree --locked --target x86_64-pc-windows-msvc --no-default-features -e features -i rusqlite`.

Before and after stdout (identical):

```text
rusqlite v0.40.2
└── rusqlite_migration v2.6.0
    └── rusqlite_migration feature "default"
        └── what-changed v0.1.0 (D:\Project\.WhatChage\src-tauri)
├── rusqlite feature "backup"
│   └── what-changed v0.1.0 (D:\Project\.WhatChage\src-tauri)
├── rusqlite feature "bundled"
│   └── what-changed v0.1.0 (D:\Project\.WhatChage\src-tauri)
├── rusqlite feature "cache"
│   └── rusqlite feature "default"
│       └── what-changed v0.1.0 (D:\Project\.WhatChage\src-tauri)
├── rusqlite feature "default" (*)
├── rusqlite feature "ffi-sqlite-wasm-rs"
│   └── rusqlite feature "default" (*)
├── rusqlite feature "functions"
│   └── what-changed v0.1.0 (D:\Project\.WhatChage\src-tauri)
├── rusqlite feature "hashlink"
│   └── rusqlite feature "cache" (*)
└── rusqlite feature "modern_sqlite"
    └── rusqlite feature "bundled" (*)
```

Devtools command: `cargo tree --locked --target x86_64-pc-windows-msvc --features devtools -e features -i rusqlite`.

```text
rusqlite v0.40.2
└── rusqlite_migration v2.6.0
    └── rusqlite_migration feature "default"
        └── what-changed v0.1.0 (D:\Project\.WhatChage\src-tauri)
            ├── what-changed feature "default" (command-line)
            └── what-changed feature "devtools" (command-line)
├── rusqlite feature "backup"
│   └── what-changed v0.1.0 (D:\Project\.WhatChage\src-tauri) (*)
├── rusqlite feature "bundled"
│   └── what-changed v0.1.0 (D:\Project\.WhatChage\src-tauri) (*)
├── rusqlite feature "cache"
│   └── rusqlite feature "default"
│       └── what-changed v0.1.0 (D:\Project\.WhatChage\src-tauri) (*)
├── rusqlite feature "default" (*)
├── rusqlite feature "ffi-sqlite-wasm-rs"
│   └── rusqlite feature "default" (*)
├── rusqlite feature "functions"
│   └── what-changed v0.1.0 (D:\Project\.WhatChage\src-tauri) (*)
├── rusqlite feature "hashlink"
│   └── rusqlite feature "cache" (*)
├── rusqlite feature "modern_sqlite"
│   └── rusqlite feature "bundled" (*)
└── rusqlite feature "trace"
    └── what-changed feature "devtools" (command-line)
```

The only added rusqlite capability is trace via explicit devtools; hooks stay
absent. Cargo.lock/pinned rusqlite and bundled SQLite versions do not change.
This resolves dependency-feature isolation only, not normal compilation,
shipped artifact eligibility or legal/notice approval. These build trees do
not register callbacks or execute database work.

Unresolved questions: final frozen source/build/lint/lifecycle verification,
owned fixture correctness and exact once-per-scale diagnostic results.

