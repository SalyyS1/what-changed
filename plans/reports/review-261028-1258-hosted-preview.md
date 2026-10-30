# Hosted Windows preview shipping review

## Scope

Read-only review of:

- `scripts/hydrate-preview-runtime.mjs`
- `scripts/verify-preview-notice-continuity.mjs`
- `scripts/prepare-public-source.mjs`
- `scripts/prepare-runner-notice-packet.mjs`
- `.github/workflows/build-windows-preview.yml`
- `docs/development-preview.md`

Reviewed against the staged public export at `.qa/github-build-preview-20261028` and the requested boundary: compile may proceed, but installer bundling/upload must remain blocked until the exact runner redistribution review is complete. No build, server, network mutation or dependency operation was run.

## Verdict

No concrete P1/P2 defect found in the bounded shipping scope.

### Runtime inventory and hydration

- Hydration uses the frozen 433-file inventory, verifies the Node executable against the pinned official checksum source, installs the Playwright-matched Chromium revision, copies only inventoried runtime/source files, then runs the closed-schema byte inventory checker.
- `runner/bundle` and runtime binaries are absent from the staged public source. The workflow reconstructs them on the hosted Windows runner.
- Exact runtime and notice inputs are marked `-text` in `.gitattributes`, preserving the reviewed bytes across Windows checkout.

### Compile / redistribution split

- The workflow uses pinned actions, read-only repository permission and a non-persistent checkout credential.
- Source/type/protocol/native authority checks precede runtime hydration and compilation.
- The runner-enabled desktop is compiled with `--no-bundle`; `check:runner-notices` is a mandatory subsequent gate. With `release/runner-notice-review.json` still pending, installer bundling and upload cannot execute.
- Desktop notices are prepared explicitly before the Tauri command (`build-windows-preview.yml:69-75`). The root Tauri `beforeBuildCommand` also prepares them, so the configured `dist-installer/THIRD_PARTY_NOTICES.txt` resource is not missing.
- Artifact upload contains only the unsigned installer, both notice payloads, preview notes, checksums and source/run evidence. The workflow creates no release, tag, Pages deployment or update feed.

### License continuity

- `verify-preview-notice-continuity.mjs` binds the reviewed baseline commit and Cargo manifest hash, removes exactly one `Win32_System_JobObjects` feature line for comparison, and rejects any other Cargo manifest change.
- It also requires unchanged package/license entries, approval evidence, Cargo lock and Bun lock. Runner notice approval remains explicitly false and separate.
- The prepared runner packet identifies Node, Playwright and the exact Chrome for Testing executable/terms without treating Chromium source licensing as redistribution approval.

### Public source projection

- Projection is restricted to an existing clone under the repository `.qa` root, rejects source symlinks, excludes dependencies, build outputs, databases, secrets, runtime bundles and binary artifacts, and scans textual candidates for credential patterns.
- Exact runner inventory/notices/legal texts are copied byte-for-byte. Private `.qa` contents, local runtime bundle and dependency caches are not staged.
- Observed path/PID matches in staged audit documents were project paths, synthetic fixtures or already-public provenance references; no personal profile path or private evidence contents were found in the reviewed scan.
- Staged source contains no `runner/bundle`, `.qa`, `node_modules`, `dist` or `dist-installer` payload.

## Known gates, not findings

- Human runtime redistribution review is pending by design; the current workflow must fail at the notice gate after compilation.
- Clean-Windows, installed upgrade, enforceable network confinement, accessibility, performance, soak and real-owner acceptance remain deferred and are stated in preview notes.
- G2/G3 and artifact-specific publication approval remain pending. A successful future workflow run is build evidence only.

## Unresolved questions

- Human decision on redistribution of the exact Chrome for Testing runtime and retained terms.

Status: DONE
Summary: Bounded hosted-preview source, CI split, notice continuity and public projection review found no supported shipping defect before branch push.
Concerns: Runtime human review intentionally blocks installer bundle/upload; external acceptance and publication gates remain open.
