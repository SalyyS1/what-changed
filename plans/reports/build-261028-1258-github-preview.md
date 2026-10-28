# Current feature build and GitHub update

Status: DONE for source update, complete runner-enabled QA artifact and nonexecuting inspection.

## Authorization and cleanup

Owner requested a new build/GitHub update and explicitly permitted removing only
reproducible build/dependency files. Verified `src-tauri/target/debug`: 4244639819
bytes / 6317 files, no tracked source, redirect or running project process.
Native PowerShell cleanup was rejected by automatic approval review: `blocked by
policy`. No alternate deletion bypass. Original source, release cache, QA backups
and personal application data preserved. A later observation found D about 4.28 GiB
free and C about 2.89 GiB; this session did not perform the rejected deletion.

## Build route

Verified public target: `SalyyS1/what-changed`, default main at
`9f8c0df94118287d989b1b253f619050aa212543`. Prepared a separate shallow clone
under `.qa/github-build-preview-20261028` and branch `codex/preview-20261028`.
Original dirty branch/history remains untouched by shipping mutations.

Source projection: 592 staged files at inspection. Excludes private QA, native
executables, browser bundles, dependencies, databases, keys and local history.
Personal account/profile paths were scrubbed from new documentation. Intended
project paths and synthetic process evidence remain. One secret-shaped source
is the inspected synthetic redaction fixture with explicit non-account values.
No raw browser HTML evidence is staged. Runtime texts/inventory have Git attributes
that preserve exact bytes, checked against index hashes.

The read-only hosted Windows workflow checks source, frontend/native authority,
generated IPC and exact runtime hydration. Native compilation precedes the runtime
notice gate. That gate blocks installer bundling/upload until the exact new review.
No automatic Release, tag, merge, Pages deployment or update feed is configured.

## License provenance and selected scope

Old reviewed desktop package choices/source terms remain unchanged: 232 inputs.
Only `Win32_System_JobObjects` was added to the existing locked Windows crate.
A continuity guard checks this exact manifest-only delta and unchanged license
entries against the public baseline. Notice preparation passes; original human
approval is retained, not replaced by a claimed new review.

Owner selected waiting for the complete runner-enabled candidate. The
[runtime review packet](../261028-1258-build-and-github-update/runtime-review-summary.md)
contains the exact 433-file inventory, complete Node/Playwright notices and full
credits/executable terms from the actual Chrome for Testing browser. Browser
redistribution review is pending; Chromium source licensing is not assumed to
cover this executable. No runtime approval or G3 is fabricated.

## Verification

- Desktop notices: 232 reviewed inputs, generated SHA256
  `aea1d355d6864858959b2a9e4c8ee628acd56718d1e6a8b2620961f344f8c6a5`.
- New continuity guard + existing inventory/release guards: 10 tests passed.
- Full Rust formatting check passed after formatting new module registrations/tests.
- Documentation check: 44 Markdown pages/links passed.
- Build workflow YAML parsed through Bun.YAML; read-only permission confirmed.
- Project code/scripts/docs staged whitespace check passed. Historical plan/test
  output and verbatim license whitespace is preserved, not a code-check pass.
- Public G0/G1 approval hashes validated after projection.
- Staged runtime inventory exact SHA256
  `ec77cefc82ae2b433b188391c36452a4302c049f6778228e53869527b3c66b1b`;
  notice payload `ee2ceffb96e1203fe96ceb31b165c0e947524f7fb5339b0ab38cfe4f01b32410`.

Independent hosted-build review and remote run receipts are recorded separately.
No successful new installer, release upload or full acceptance is yet claimed.

## Source push and hosted follow-up

- Source commit `c4830d8f1392f30a924a3e76558ace1332cd7f91` is on
  [draft PR #1](https://github.com/SalyyS1/what-changed/pull/1); public main was preserved.
- GitHub initially rejected the synthetic Slack fixture. No bypass was used.
  Fixture strings are assembled with Rust `concat!`; a standalone Rust comparison
  confirmed all five fields of all twelve fixtures are byte-identical to the prior
  literals. This proves fixture parity, not a new full redaction-suite pass.
- [First run](https://github.com/SalyyS1/what-changed/actions/runs/40646188908)
  passed typecheck/rustfmt/desktop continuity, then failed four inventory tests at
  the lexical-root versus realpath-entry containment check on hosted Windows.
- Commit `2517f54f01829caf29f9f44bbbface9bb4976174` resolves the root after auditing
  its original ancestors. Containment now compares canonical paths and rejects
  parent-relative or absolute escapes. New junction-root/ancestor/nested escape
  tests and existing inventory/release tests passed: 12/12 locally.
- [Follow-up run](https://github.com/SalyyS1/what-changed/actions/runs/40666073479)
  passed source/frontend checks and entered native tests at observation. Final
  result remains pending.
- Public documentation now preserves the historical preview navigation and passes
  the relative-link check: 45 pages. Independent shipping review found no P1/P2 defect.
- The owner selected verifying the current browser first. The
  [official-source report](research-261028-1638-browser-redistribution.md) found no
  distribution grant and a prohibition in the applicable public terms. A separate
  applicable grant or owner decision to select a compatible redistributable browser
  is pending. Neither a replacement nor notice approval is inferred.

## Native short-path repair

The second hosted run proved the JavaScript alias case with a receipt mapping
`C:\Users\RUNNER~1` to `C:\Users\runneradmin`, then passed 12 inventory/release
tests, 15 runner tests, five frontend test files, native idempotency (2) and
generated bindings parity (1). Lifecycle tests failed while opening the managed
store: the native handle validator compared a normalized final path with the
lexical 8.3 ancestor. [Failure excerpt](build-261028-1258-native-alias-ci.txt).

Commit `0265cc58855410f0c42cfd1fff79c6cde1d7c1c7` expands the expected name with
`GetLongPathNameW` and rejects expansion failures. Final handle-path matching,
reparse/hard-link rejection and sharing modes remain enforced. Advisory counsel
supported this repair. The attempted incremental independent review could not
run due to model capacity; controller diff review and verification were performed.

Local verification:

- Managed file-lock tests: 9/9, including real short-name API, wrong-object,
  pinned-parent, substitution, rename, hard-link and junction checks.
- Preview lifecycle: 3/3.
- Structured/log secret-shape redaction: 2/2, exercising the unchanged compiled
  synthetic fixtures.
- Rust formatting, desktop notice continuity and 45-page documentation checks passed.

The [third hosted run](https://github.com/SalyyS1/what-changed/actions/runs/40667979812)
passed those checks, frontend production build and exact old-runtime hydration.
Desktop notice preparation then failed: Git had normalized approved CRLF license
texts in generic-array, nu-ansi-term and quick-xml to LF. Their original bytes and
approval hashes were preserved locally.

## Upstream license byte preservation

Commit `8b7284614491d7313c312369be87b1e60d97c25c` marks vendored
`legal/third-party/**` as `-text`, preserves exact upstream bytes through public
projection, and restores only the three line-ending transformations. All 268
primary/notice records (232 desktop entries including native license records)
match approved hashes in both the publication worktree and Git index. Ignoring
line-ending whitespace leaves no license-content diff; review choices are unchanged.

Notice continuity passes. Direct notice generation in the unbuilt clone reported
missing `dist-installer/frontend-inputs.json`; that file is generated by the hosted
`bun run build` step before notice preparation. This prebuild probe does not prove
the hosted result. Advisory counsel supported the repair. The
[fourth hosted run](https://github.com/SalyyS1/what-changed/actions/runs/40671233391)
passed desktop notice generation and runner-enabled release compilation. It then
failed at the mandatory runtime-owner-review gate with the explicit pending-review
message. This verifies the EOL repair and compilation, not a completed installer.

## Replacement-browser authorization

The owner explicitly selected a Chromium artifact with suitable redistribution
rights. This authorizes selection, verification and integration, not a human
review of an unseen new inventory or obligations. Official snapshot 1654400 passed
bounded runner compatibility checks, but its embedded credits are a sample
placeholder and its statically linked FFmpeg source-delivery route is unresolved.
It is not a cleared distribution replacement. Complete notice/source evidence is
required before freezing the new runtime packet.

## Approved replacement packet

Ungoogled Chromium 156.0.8078.47-1.1 passed eight actual runner actions/nine
assertions, including role isolation, authorized screenshot, close and abort.
Complete credits retain 497 license blocks. The Microsoft SDK D3DCompiler side-copy
is excluded; on the Windows 11 test host the GPU process loaded System32's DLL and
the same runner checks passed. Clean Windows 10/11 acceptance is still open.

Local runtime: 197 files / 543579634 bytes; inventory SHA256
`cc3bb9c020ec053b11df341d925f3f25e26a6dc9f0cfbdb5ab1d1cf3d3ec5554`.
The exact native loader check passed. Acquisition/extraction/source packet tests
passed 33/33; inventory/release/continuity 12/12; runner source tests 15/15.
Independent integration review found no concrete P1/P2 issue in the scoped change.

The owner answered **“Chấp thuận hồ sơ Chromium, tiếp tục build QA”** for the
frozen proposal. [Approval evidence](../../release/runner-notice-approval.json)
binds inventory, complete notice payload and proposal hashes. The local exact-byte
notice guard passes. This approves installer QA conditional on the hosted source
gate, not public v1 or an artifact release.

All seven corresponding-source archives (~2 GB), pinned patches/build scripts and
relink instructions must be in the same Actions download as the installer. Hosted
hash/member checks run before the approval/bundle/upload gates. The large Chromium
source archive was not downloaded locally; the four small target-source overrides
and their required members were validated. No source completeness claim rests on
minimal member samples alone; the pinned exporter and supported build recipe own
the source-route evidence.

## Source archive verification and packet repairs

The first replacement build passed native checks, exact runtime loading, notices
and release compilation, then Windows `tar.exe` rejected an archive filename.
Installer bundling/upload remained blocked. The source inspector now uses Python
3.12.10 streaming tar metadata, supports Unicode/PAX names and fails on malformed
or truncated later entries. Acquisition/archive/source tests passed 40/40.

[Run 40729481803](https://github.com/SalyyS1/what-changed/actions/runs/40729481803)
verified all seven whole-file source hashes and required archive members. It was
cancelled after that successful source step to build the final integration commit;
the cancelled run is not a completed installer result.

The local release route now creates the same combined installer/source packet,
the hosted verifier checks both desktop and runner NSIS notice resources, and
build evidence is included in recursive checksums. Independent scoped review
found no remaining concrete P1/P2 issue after the local co-delivery repair.

Current build source: `e5906ea6ae2fb06859d7b361df00b36425dd496c`.
[Run 40730363355](https://github.com/SalyyS1/what-changed/actions/runs/40730363355)
passed source/frontend contracts and all seven source hashes/member checks at
`2026-10-28T17:03:40Z`, then completed native checks, release compilation, exact
runtime review, NSIS packaging, both notice checks and artifact upload successfully.

## Successful candidate and inspection

[Complete artifact 12838120354](https://github.com/SalyyS1/what-changed/actions/runs/40730363355/artifacts/12838120354)
contains the unsigned installer and all seven corresponding-source archives.
Installer SHA256: `70ced3ac9dd66abd4235d525b7418b0d9f5d147f05e9c652e1b34c619001efbc`.
Local selective download verified ten file hashes, source descriptor/member receipts
and co-delivery metadata without downloading large sources again. Fresh nonexecuting
extraction verified the product against its embedded installer payload, all 197
runtime files and both notices. No excluded SDK compiler DLL was present.
Installer/application Authenticode status is `NotSigned`.

The [inspection report](qa-261029-0042-runner-installer-artifact.md) owns exact
hashes, retention and Azure-range/7-Zip listing qualifications. Source/archive
bytes were verified on CI; the complete ZIP digest was not locally recomputed.
The private selective QA copy is not a separate delivery packet.

Unresolved for public release: artifact-bound approval and the installed/clean-Windows/
network/accessibility/performance/soak/real-owner acceptance gates.
