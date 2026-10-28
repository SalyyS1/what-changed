# Ownvero implementation preflight

Recorded 2026-10-27. Local feasibility evidence; not release approval.

## Source and ownership

- HEAD `13d60df4c98fd50532c3d4a338fd6eb7be6c0db8`; work continues on
  `codex/ownvero-owner-workspace` with all prior dirty edits retained.
- Local baseline `.qa/ownvero-preflight/source-baseline.json` records 205 changed or
  untracked files at snapshot time, including new preflight files. SHA-256
  `52230db59b46f8d7aba9533095549b34c3356b0e7fb1ace46cfa4b95f34d43ba`.
  This is a rehearsal identity, not a clean release commit.
- No personal app database, dotenv or browser profile was read or changed.
- Existing process inventory showed no running project dev server or installed app;
  unrelated Node/Codex/terminal processes were left alone.
- Referenced `.Codex/rules` Markdown files are absent. Matching rules were read from
  `.claude/rules`, alongside the supplied repository instructions.

## Decisions and entry gates

[G0/G1 user evidence](approval-g0-g1.json) records the exact question and answer.
[Approval ledger](../approval-ledger.json) binds those decisions to the four scope
contracts. G2 public identity and G3 frozen release approval remain pending.

[Approval guard](../../../scripts/check-ownvero-approvals.mjs) rejects pending,
unknown, stale and inconsistent records. It records conversation provenance; it is
not a digital signature and cannot authenticate a fully rewritten repository.

User subsequently approved proceeding with P02–P06 while retaining clean-Windows
runtime validation and enforceable network restrictions as mandatory P07 entry gates.
See [sequence decision](approval-offline-sequence.json). This changes scheduling;
neither native gate has passed.

## Runtime feasibility on this host

Owners: [bundle preparation](../../../runner/spikes/prepare-runtime-bundle.ps1),
[browser smoke](../../../runner/spikes/run-packaged-runtime-smoke.ps1),
[native lifecycle harness](../../../runner/spikes/test-owned-process.ps1).

- Non-elevated Windows 11 build 26200 x64, already inside a harness Job Object.
- Node v24.12.0 executable matched the official SHA-256 inventory. Playwright 1.62.1
  uses matched Chromium revision 1234 / browser 151.0.7922.34, downloaded into QA.
  Existing host Chromium revision 1223 was deliberately not used for this evidence.
- Bundle inventory: 432 files, approximately 555 MB installed. A compression sample
  was 245,711,402 bytes. This is a ZIP sample, not an NSIS installer or distributable.
- Explicit environment, empty PATH, no inherited Node options/module path or proxies.
  Browser sandbox enabled; private pipe, no TCP debugging server or personal profile.
- Controlled booking fixture: UI creates synthetic booking + new field, separate
  staff session reads committed JSON file, another customer receives 403 without
  selected record disclosure, target-reported revision matches before/after.
- Initial browser smoke passed four assertions in 9,378 ms, exit 0, zero remaining
  Job processes and no remaining Playwright profile entries. Final helper revision
  requires the final browser rerun recorded in the verification evidence.
- Native tree cases: normal exit, cancel, disposal and abrupt parent death with nested
  Jobs. The harness retains process handles and checks all owned Node/conhost members.
  Latest final-helper run passed; detailed local result is
  `.qa/ownvero-runtime-spike/owned-process-test/latest-result.json`.

This does **not** prove a clean Windows installation, installer packaging, Chromium
crash/cancel cleanup, network confinement, remote support or full runner acceptance.
No Windows Sandbox/VirtualBox/VMware command was available in the inspected PATH.
No remote user target, payment/email integration or production data was exercised.

## Resources and cleanup

D: initially had 2,194,567,168 free bytes; after the QA bundle and compression sample,
the observed free space was 1,374,482,432 bytes. Plan a build budget before Rust/NSIS.
An automatic policy review rejected direct recursive debug-cache deletion, so that
deletion did not run. Release artifacts and profile backups remain intact.
No phase claims a successful cleanup or reclaimed space from that rejected command.

## Old obligations

[Complete machine-readable ledger](old-gates.json): 52 unchecked phase requirements
and 19 unchecked index roll-ups; roll-ups are not additional implementation tasks.
61 physical rows carry candidate/release evidence forward; 10 retain specific legacy
behavior. Every original row remains open, with original owner retained. No waiver or
supersession is inferred from the pivot.

Latest repaired installed 0.1.0 candidate is documented in
[installed QA](../../reports/qa-261027-0619-repaired-installed-candidate.md).
It predates the final System Map source repairs. Its installer is not an Ownvero
candidate and cannot prove current-source installed acceptance.

## Remote site reconciliation recipe

Read-only GitHub checks found public `SalyyS1/what-changed`, Pages workflow hosting
at `https://salyys1.github.io/what-changed/`, upstream main
`9f8c0df94118287d989b1b253f619050aa212543`. Local Git has no configured remote.
The four remote-only site/workflow files were fetched at that exact commit into
`.qa/ownvero-preflight/upstream-site/`; Git blob SHA-1 and SHA-256 were checked.
The matching inventory lives in `.qa/ownvero-preflight/upstream-site-manifest.json`.

P11 should import those exact files into currently absent paths, preserve legacy
hash anchors (`overview`, `first-investigation`, `timeline`, `workspace`, `privacy`,
`build`, `status`), then review the site diff. The existing workflow uploads the raw
wiki folder on main; replace its output route only when the new build/allowlist/G3
guard exist. Do not copy an active deploy workflow early or overwrite dirty app files.
Keep repository/Pages URLs and use a reviewed branch merge, never a force push.
No GitHub content was written and no deployment ran during preflight.

## Vertical example and continuation

Owner intent: add booking note while preserving staff visibility and customer access
boundaries. Candidate is the fixture's explicit revision marker, not its URL. Named
observations are separate from the owner's future accept/reject/defer decision.
The smoke does not manufacture an owner acceptance record or market validation.

P02–P06 may now implement the offline workflow under the approved sequence. P07
remains closed until clean-Windows and network-boundary evidence is reviewed.
G2/G3 and early participant rehearsal remain separate future decisions/evidence.

## Unresolved questions

- Which clean Windows environment will supply the P07 packaged runtime proof?
- Can the proposed non-admin remote egress boundary meet all deny cases? Not yet tested.
