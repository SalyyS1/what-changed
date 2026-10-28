# Current feature build and GitHub update

Status: source pushed to draft PR #1; hosted native build in progress; installer review open.

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

Unresolved questions: exact browser redistribution permission and runtime human
review; artifact-bound publication approval after build.
