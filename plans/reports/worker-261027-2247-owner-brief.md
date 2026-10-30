# P05 deterministic owner brief report

Status: DONE_WITH_CONCERNS

Implemented `src-tauri/src/owner_workspace/brief.rs`, DTO additions in `src-tauri/src/dto/owner_workspace_dto.rs`, one-snapshot SQL helper in `src-tauri/src/owner_workspace/storage/repository.rs`, and temporary fixture tests in `src-tauri/tests/owner_brief.rs`.

API to wire:

```rust
OwnerStore::render_brief(
    app_id: &str,
    change_id: &str,
    selection: OwnerBriefSelectionInput,
) -> AppResult<OwnerBriefPreviewDto>
```

`OwnerBriefSelectionInput` controls scenarios, candidate identity, environment details, and limitations. `OwnerBriefPreviewDto` returns deterministic escaped Markdown, byte count, estimated tokens, omitted sections, and limitations. It rejects unknown scenario revisions and output over 32 KiB with selection guidance. It never reads clipboard or contacts a target. The default omits candidate identity and local URL/origin detail; no-run and unresolved evidence states are explicit. Stored text is scrubbed again through the existing Sanitizer before rendering.

The renderer reads app, change, confirmed rules, candidate/environment, and scenario revisions in one owner SQLite read snapshot. SQL remains under owner_workspace/storage. Portable JSON/ZIP import/export and decision memory are intentionally not implemented in this phase.

Validation: `rustfmt --edition 2021 --check` passed for all owned source and brief test files. Cargo was not run because root reserved shared Cargo build/disk. Parent must wire DTO module, central IPC command and run `cargo test --manifest-path src-tauri/Cargo.toml --test owner_brief`.

Unresolved questions: none beyond central integration and full P05 portability scope.

Follow-up: fixed storage helper visibility (storage::brief_snapshot reexport), explicit Vec<String> limitations inference, and removed unused DTO import; rustfmt check passes.
