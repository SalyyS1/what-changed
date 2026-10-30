# Desktop release readiness — 2026-10-30

Status: DONE_WITH_CONCERNS. Build and source inspection advanced; public release remains NO-GO.

Latest state: the owner approved the exact runtime source delta on 2026-10-30.
Replacement build `41091508412` and nonexecuting installer inspection passed on
2026-10-31, Asia/Saigon, for public source `75dcb67c`. The active freeze, preflight,
notices and desktop continuity pass. The initial audit below records the earlier
blockers; follow-through and current artifact evidence appear near the end.

## Scope and source

- Audited local `65ecf83a5d90a2c2118d8798d70aaad3f59f100b` on `codex/ownvero-owner-workspace`.
- Read root README, development/deployment guides, phase 12, deferred acceptance ledger, candidate packet, release guards, hosted workflow, latest artifact QA and verification continuation.
- Initial audit made no app/site/config/approval edits, builds, installer execution, large downloads, publication or user-data access. Later explicitly authorized preflight and approval work is recorded below. Temporary test fixtures were handled by existing tests.
- Repository `AGENTS.md` and `C:/Users/<user>/.Codex/rules/development-rules.md` are absent; supplied session instructions apply. Applied installed `ak-test` skill.

## Prior hosted artifact: initial audit snapshot

At the initial audit on 2026-10-30, GitHub API confirmed the latest preview run was
[40730363355](https://github.com/SalyyS1/what-changed/actions/runs/40730363355):
completed/success, source `e5906ea6ae2fb06859d7b361df00b36425dd496c`, branch
`codex/preview-20261028`, created `2026-10-28T16:53:17Z`.

| Item | Current evidence |
|---|---|
| Complete artifact | [12838120354](https://github.com/SalyyS1/what-changed/actions/runs/40730363355/artifacts/12838120354), `what-changed-owner-workspace-preview-8` |
| Size | 2,227,488,699 bytes |
| API ZIP SHA256 | `4a9b9f4c462fd851951ae763b27756318ae7f2d2e50435b82bbd80a009be8ad7` |
| Retention | Not expired; expires `2026-11-11T17:31:56Z` |
| Prior installer measurement | 175,106,389 bytes; SHA256 `70ced3ac9dd66abd4235d525b7418b0d9f5d147f05e9c652e1b34c619001efbc` |
| Prior native executable measurement | 18,817,024 bytes; SHA256 `74040788006faab497d9aee93e6985013f7debfa76c6f4bb124084dad0594df9` |

The live API digest/size/source/expiry match the
[nonexecuting artifact QA](qa-261029-0042-runner-installer-artifact.md).
Installer/executable hashes above remain that report's measurements; bytes were
not downloaded or rehashed in this audit. Complete ZIP was not rehashed locally.
All seven matching source archives, notices and build/relink instructions belong
to the complete artifact. Retain/co-deliver them; the selective local installer
copy is not the distribution packet. Current HEAD has no matching hosted build.

## Commands and results

| Command / check | Result |
|---|---|
| `node scripts/check-owner-release.mjs --candidate release/candidate-evidence.json` | Exit 0; structured **NO-GO**, 15 missing keys, `publicationApproved: false` |
| `node scripts/check-versions.mjs` | Pass; package/Cargo agree at `0.1.0` |
| `node scripts/check-ownvero-approvals.mjs --gates G0,G1,G2,G3` | Exit 1; G2 approval missing. Ledger has G0/G1 approved, G2/G3 pending |
| `node scripts/check-runner-bundle.mjs runner/bundle` | Pass; 197 files, 543,580,088 bytes; inventory SHA256 `31629890ecec49637621c5a4a321085306ac4392cbd95619598a81c699c47299` |
| `node scripts/check-runner-notices.mjs` | Exit 1; `Runtime bytes changed after notice review` |
| `node scripts/verify-preview-notice-continuity.mjs` | Exit 1 locally; reviewed Git history object missing. Remote baseline comparison below passes |
| Existing owner-release/bundle/continuity/download/NSIS guard tests | 55/55 pass, zero skips, ~34.6 s |
| `node scripts/check-runner-policy.mjs` | 15/15 pass, zero skips, ~34.2 s |
| `node scripts/check-docs.mjs` | Pass; 46 Markdown files and relative links |

Guard suite command:

```powershell
node --test scripts/check-owner-release.test.mjs scripts/check-runner-bundle.test.mjs scripts/verify-preview-notice-continuity.test.mjs scripts/fetch-reviewed-runtime-artifact.test.mjs scripts/verify-nsis-notice-copy.test.mjs
```

These source/packaging checks prove their assertions. They do not prove installed
browser confinement, disabled-mode behavior, native acceptance or participant value.
The [2026-10-29 verification report](verification-261029-1149-owner-workspace-continuation.md)
records earlier frontend/build checks; its Rust attempt did not complete due to
MSVC PDB/resource failures. No new Rust result is claimed here.

## Exact source/inventory distinction

Frozen reviewed inventory SHA256 is
`cc3bb9c020ec053b11df341d925f3f25e26a6dc9f0cfbdb5ab1d1cf3d3ec5554`
at 543,579,634 bytes. Current local bundle differs in exactly one owned file:

| File | Frozen | Current |
|---|---|---|
| `src/policy.mjs` bytes | 4,058 | 4,512 |
| SHA256 | `ee1c8922d8c990ecef9b829ad87f2857c7eadc38dfa7d0c2a194f6fcc4ac38e6` | `0a4cd02e3153754ceb133cc76f5eab9025ce470508fb03687ce66df5c0a0c912` |

All **186 third-party files** have identical path, byte count and SHA256 in both
inventories; no vendor file added/removed. All reviewed license/source files and
the packaged notice payload still match their reviewed hashes. Both review and
approval currently record the aggregate inventory hash, including owned JS.
Thus the existing notice guard fails on the owned source delta; evidence does not
show a changed Node, Playwright, Chromium or license payload.

`hydrate-preview-runtime.mjs` reads the old frozen inventory, then copies current
`runner/src`. Its final inventory guard will reject current `policy.mjs` against
the old record. This occurs after large downloads; do not dispatch that unchanged
route expecting a successful current-source build.

### Smallest concrete controller work

1. Preserve the approved frozen inventory and notice/approval payloads. Add a
   narrowly tested continuity route that derives current owned `src/` records,
   preserves every approved vendor record exactly, and checks source parity.
   Notice validation can compare current vendor records against the original
   snapshot whose full hash is already approval-bound. Reject any vendor,
   license/source-material, version or notice change. Actual changes to reviewed
   runtime bytes/terms require review; an owned-source-only change is not evidence
   of changed third-party terms. Do not rewrite a human decision record.
2. Add a pre-download inventory/source preflight to the hosted hydration route,
   or otherwise validate these derived inputs before downloading/compiling.
   Keep the actual complete inventory hash in new artifact provenance.
3. Ensure the local continuity gate can read its pinned reviewed history before
   running local release. This is checkout preparation, not a dependency change:
   local Git is not shallow but lacks both preview history objects. Hosted
   `fetch-depth: 0` can retain the existing preview branch history. A targeted
   fetch of that branch is an operator remedy; no fetch was performed here.

The pinned reviewed baseline commit
`9f8c0df94118287d989b1b253f619050aa212543` exists on public GitHub. A read-only
contents API comparison established: original Cargo hash matches
`5d64037750d5a074843ed2d4f46d1ec65ac709ec5d8825210c7a0f214ecda65d`;
only existing `Win32_System_JobObjects` differs; reviewed package entries,
approval evidence, Cargo lock and Bun lock are unchanged. Do not infer a new
license obligation from the missing local history object.

## Remaining real release evidence

`release/candidate-evidence.json` is intentionally unfrozen, still references
`13d60df4c98fd50532c3d4a338fd6eb7be6c0db8`, and has empty artifact/verification
arrays. Its 15 missing keys identify absent frozen evidence; they are not 15
newly observed product failures:

- Clean source freeze, matching installer and approved download manifest.
- Automated evidence bound to that candidate; native install/upgrade and
  privacy purge/restore.
- Clean non-admin Windows with packaged runtime and no developer dependencies;
  disabled runner outbound/child absence; authorized egress deny cases;
  cancel/parent-crash/sleep/timeout/descendant lifecycle.
- Native accessibility, scaling, forced colors, screen reader and map fallback.
- Named-machine performance and four-hour lifecycle soak.
- Real-owner repeat use under the pilot protocol; participants/outcomes remain
  absent. Source fixtures cannot close this row.
- Current license continuity, rendered site evidence and exact G2/G3 approval.

Unsigned status, retained `What Changed` data identity and unconfigured automatic
updates are documented limitations; this audit adds no certificate/feed blocker.
G2 production identity and G3 exact frozen publication scope remain pending.

## Recommended build route

Use the existing bounded hosted Windows preview workflow after source/inventory
guards are repaired and reviewed inputs retained. It has already produced a
complete runner/source packet and avoids the recorded local Rust resource/disk
failures. Freeze and record the intended source revision; trigger its documented
preview build route without creating a Release/tag or enabling a download CTA.
Validate all seven source archives, packaged inventory, notices, generated NSIS
script and artifact receipts as the workflow already requires.

Then run the native/clean-Windows/soak/participant checks on one exact candidate;
populate source-bound evidence and the manifest only from real results. Exact
G2/G3-bound approval is still required before binary/site release publication.
Preserve the existing complete QA artifact/source access before 11 November
retention expiry if it is still needed.

## Approved runtime delta follow-through

On **2026-10-30, Asia/Saigon**, the user answered:
“Chấp thuận delta, tiếp tục build QA”. This approves the exact proposed runtime
delta for an unsigned QA build; public release and G2/G3 remain unapproved.

- [New decision evidence](../../release/runner-source-delta-approval.json), SHA256
  `4aacd57ec5bafe2ac0f5e0dadeceee33a29d3269506411447b64e041b385880d`.
- Exact two-file scope: `095020f04c91eabf7e23c7c5e46b26c6e6fbb593f7504a24b8e20927360ba78a`.
- Active frozen inventory now equals the approved proposal exactly:
  `31629890ecec49637621c5a4a321085306ac4392cbd95619598a81c699c47299`;
  197 files / 543,580,088 runtime bytes; all 11 owned source files match.
- Original approval remains unchanged. Exact previous inventory and review bytes
  are preserved in `release/history/runner-runtime-inventory-2026-10-28.json` and
  `release/history/runner-notice-review-2026-10-28.json`. The active review points
  to the new decision; vendor records, notices and seven source duties are unchanged.
- Approved proposal/report remain immutable as the reviewed packet. Their
  proposal-state wording records the question as asked; new decision evidence
  records the subsequent approval.

| Requested gate after refresh | Result |
|---|---|
| `node scripts/check-preview-runtime-inputs.mjs` | Exit 0; 11 source files / 40,681 bytes; exact approved inventory |
| `node scripts/check-runner-notices.mjs` | Exit 0; exact inventory and reviewed notice delta match; publication separately gated |
| `node scripts/verify-preview-notice-continuity.mjs` | Exit 0; 232 reviewed packages; existing JobObjects feature only |
| Approval/reference/scope rehash | All active and preserved references match their recorded full SHA256 |

The preflight's seven tests pass. CI now checks frozen source inputs before Rust,
dependency, source archive and runtime preparation. No hosted build had been
dispatched at that checkpoint; the later controller pushes and replacement build
are recorded below. Candidate packet/native/soak/pilot/publication evidence remains open.

Exact approved inventory JSON uses CRLF. The active release path already has
`-text` attributes. The plan proposal also needs `-text` before staging, otherwise
Git normalizes its approved bytes. Raw `git diff --check` flags CR as trailing
whitespace on the active hash-bound inventory; do not normalize those bytes.
Rehash approved inputs and evidence from the staged/committed blobs before push.

## Established safe hosted source route

The existing clean isolated clone is
`D:/Project/.WhatChage/.qa/github-build-preview-20261028`, branch
`codex/preview-20261028`, origin `https://github.com/SalyyS1/what-changed.git`.
At route discovery, both clone and live branch identified
`50b32fc88abbcbf80acc4d13965ef151c2e7ed71`. Use the existing
`scripts/prepare-public-source.mjs` projection after the controller commits the
intended primary source. It preserves public history/review records and excludes
private QA, local repository objects, runtime binaries and secret-bearing files.
This avoids pushing the primary repository's blocked history.

Controller sequence, after checking clone status and exact source/approval hashes:

```powershell
$previewClone = 'D:/Project/.WhatChage/.qa/github-build-preview-20261028'
git -C $previewClone status --short
git -C $previewClone fetch origin codex/preview-20261028
git -C $previewClone merge --ff-only FETCH_HEAD
node scripts/prepare-public-source.mjs $previewClone
node "$previewClone/scripts/check-preview-runtime-inputs.mjs"
node "$previewClone/scripts/verify-preview-notice-continuity.mjs"
git -C $previewClone diff --stat
git -C $previewClone add --all
git -C $previewClone diff --cached --stat
# Inspect staged source and verify all approval-bound blob hashes before commit.
git -C $previewClone commit -m "build: refresh approved owner workspace QA source"
git -C $previewClone push origin HEAD:codex/preview-20261028
```

Record the primary-to-public commit mapping. That final push triggers the preview
workflow for these changed release/script inputs; avoid a duplicate manual
dispatch. It uploads a review artifact only. Complete source co-delivery, native
acceptance and exact public-release approval are still separate requirements.

## Hosted build continuation

The approved source projection `f9ac3005e7428b51af0f2ca9907015b90576af26`
started [run 41086258362](https://github.com/SalyyS1/what-changed/actions/runs/41086258362).
Frozen runner preflight, source contracts, focused frontend integration, all seven
source archives, native authority/generated contracts, runtime hydration and the
native packaged-inventory loader passed. The production frontend built, then
desktop notice generation stopped on `generic-array` license hash mismatch.
The failed run uploaded no artifact and produced no verified installer.

Exactly three desktop license files had LF in the primary source while the
existing reviewed hashes and successful public baseline bind CRLF: generic-array,
nu-ansi-term and quick-xml. All other reviewed desktop entries match. Restoring
the exact reviewed blobs from public `50b32fc88abbcbf80acc4d13965ef151c2e7ed71`
repairs the bytes; no terms, license choice or human approval changes. The
existing `legal/third-party/** -text` attribute preserves them thereafter.

Primary repair commit `d71eaf3` restores those exact bytes and adds the read-only
notice preflight before expensive CI work and after public projection. Combined
early tests pass 13/13. Repeating the actual frontend build and desktop notice
generator locally succeeds: 232 reviewed inputs, notice SHA256
`aea1d355d6864858959b2a9e4c8ee628acd56718d1e6a8b2620961f344f8c6a5`.
That is local generator evidence, not a hosted installer payload hash.

Replacement public source `75dcb67c3ba9c6e8c6ad97e2166dd1626c44cb11`
started [run 41091508412](https://github.com/SalyyS1/what-changed/actions/runs/41091508412).
Its new early source/notice gate passed. Final artifact evidence is owned by the
[QA inspection record](qa-261030-2235-runner-installer-artifact.md); do not reuse
the failed run as a successful candidate.

The replacement finished successfully at `2026-10-30T17:09:51Z` (31 October,
00:09:51 Asia/Saigon). Complete artifact `12995563059` contains the installer,
notices and all seven matching source archives. It is 2,227,565,645 bytes; API
retention expires `2026-11-13T17:09:27Z`. Static inspection independently checked
the installer checksum, native executable, packaged 197-file runtime inventory
and notices. Source co-delivery was checked through the ZIP directory and hosted
receipts; the complete ZIP and 2 GB sources were not downloaded/rehashed locally.
No installer or product execution is claimed by this inspection.

## Public runner packet contract gap

The current approved QA distribution keeps installer plus seven source archives
in one downloadable Actions artifact. That complete prior packet is
2,227,488,699 bytes. The website manifest accepts only an exact `.exe`, with a
2 GiB ceiling; owner-release assessment binds its hash/size to the installer.
Those contracts cannot represent the complete runner packet.

GitHub documents a per-Release-asset limit under 2 GiB, so the existing full ZIP
also cannot be copied unchanged to one Release asset. See
[GitHub release limits](https://docs.github.com/en/repositories/releasing-projects-on-github/about-releases).
This is a measured format/hosting mismatch, separate from native acceptance.
Keep downloads unavailable; a public distribution format requires an explicit
scope decision and matching manifest/schema/evidence binding before G3. Do not
silently omit sources or replace the approved source duties with a written offer.

The [concrete public download proposal](../261030-1908-website-overhaul-release-readiness/public-download-proposal.md)
recommends one Release with installer and corresponding-source ZIP, each below
the host's individual-file limit, and coordinated manifest/evidence checks.
This proposal leaves the approved complete QA Actions artifact unchanged. A new
decision is pending; no public-format code or publication approval was inferred.

## Unresolved questions

- Which clean Windows test machine and consenting pilot owners will supply the
  remaining acceptance evidence? No participants or clean host were supplied.

## Native candidate smoke follow-through — 2026-10-31

The user-authorized current-account smoke of the extracted runner-enabled
candidate passed launch, onboarding/home, synthetic `My apps` creation, clean
exit, relaunch persistence and original-app resume. The original profile was
restored byte-for-byte (13 files / 1,085,434,822 bytes) before the legacy app
restarted. See [native candidate smoke](qa-261031-native-candidate-smoke.md).

This is bounded candidate evidence only. It does not prove installer or upgrade,
clean Windows, non-admin isolation, disabled/authorized egress, accessibility,
four-hour soak, or real-owner repeat use. G2/G3 remain pending.

## Approved public distribution shape

The user approved the proposed same-Release two-asset shape after the format
mismatch was measured: one installer plus one corresponding source archive, each
with its own exact URL, byte count and SHA-256, bound to the same version/tag and
source commit. The schema, validator, owner assessment and download page now
support this shape. The checked-in manifest remains `status: unavailable`; no
Release, tag, download CTA or G3 approval was created by this implementation.
