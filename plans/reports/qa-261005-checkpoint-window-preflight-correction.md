# Checkpoint-window preflight correction — 2026-10-05

First preflight exits1 before Cargo or the workload starts. Cwd src-tauri;
the ancestor guard incorrectly checks relative `.qa` instead of the workspace
ancestor. All source/test/artifact assertions remain unchanged. Root stops
and obtains counsel before correcting only that path to the absolute
workspace `.qa`. The same preflight then admits the already approved single
10k smoke. This is not a workload rerun; no diagnostic ran on the first attempt.

Failed command fragment:

```text
foreach ($taskPath in @('D:\Project\.WhatChage','.qa','D:\Project\.WhatChage\.qa\linker-temp')) {
    $taskItem=Get-Item -LiteralPath $taskPath
}
```

Complete failure output:

```text
Get-Item: 
Line |
   2 |  … qa\linker-temp')) { $taskItem=Get-Item -LiteralPath $taskPath; if (($ …
     |                                  ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
     | Cannot find path '.qa' because it does not exist.
```

Counsel GO: substitute only the absolute workspace ancestor, repeat unchanged
guards, then run the existing canonical target once. No source, fixture,
threshold, feature, ownership or acceptance change. [Ledger](qa-261005-checkpoint-window-capture-ledger.md).
