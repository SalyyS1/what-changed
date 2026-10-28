# Native notice source correspondence — 2026-10-05

Outcome: independently verified SQLite source and WebView2 SDK candidates;
new source texts remain pending, not linked to a verified current review snapshot.
Retrieved/checked 2026-10-05 02:21 UTC. No native binary is executed or installed.
No selected-license, obligation, reviewer, draft or release-manifest field changes.
[Prior bounded ledger](qa-261005-third-party-source-ledger.md) retains the open audit.

## SQLite exact source

The packaged libsqlite3-sys 0.38.2 amalgamation declares SQLite 3.53.2 and
source ID `2026-06-03 19:12:13 d6e03d8c777cfa2d35e3b60d8ec3e0187f3e9f99d8e2ee9cac695fd6fcdf1a24`.
Local source size 9,507,037 bytes/SHA-256
`0a409f1633283fa31a9126b11fbfd64a1991c5d30defad07e5745d4667f5e23d`
match the draft. Independently calculated SHA3-256
`44fd61b9f93b4155105cb2d80c957ae6c64a8b5bd6ed51a4992f0dbd438e4e11`
matches the [official 3.53.2 release page](https://sqlite.org/releaselog/3_53_2.html).
This establishes correspondence of the actual source bytes, not just a version label.

SQLite's [official copyright page](https://sqlite.org/copyright.html) distinguishes
public-domain deliverable library code from potentially differently licensed
build scripts. The local notice is the complete first copyright-comment block
from that verified amalgamation, not a copy of the full website or wrapper MIT
license. Source slice: 426 bytes, SHA-256
`39fc4986b224f2f5575dec58ae66c7c9ee2d5d3884ba4971c932f1fd5b40b033`.
[Local notice](../../legal/third-party/native/SQLite/3.53.2/PUBLIC-DOMAIN-NOTICE.txt)
is a transcription with a final newline; its local identity is checked separately.
Local slice: 427 bytes, SHA-256
`49cf7bcb103eb3244150a23bb23d840370a4762914faf1b450072a4b03c14670`.
This technical correspondence is not an obligation or final-artifact review.

## WebView2 SDK exact packaged candidates

Read-only Windows FileVersionInfo on the packaged x64 loader reports
FileVersion/ProductVersion **1.0.3650.58**, Microsoft Corporation. This is
binary-resource evidence, not execution. The crate VCS record is
`b74dc5e2b394044bea5191052868ce7a106c202c`, path `crates/bindings`.
Local build.rs selects x64 for x86_64; src/lib.rs selects static loader linkage
for MSVC. Source selection is not proof of retention in a frozen final binary.

The [official versioned NuGet package](https://www.nuget.org/packages/Microsoft.Web.WebView2/1.0.3650.58)
was fetched into memory only from its
[exact package endpoint](https://api.nuget.org/v3-flatcontainer/microsoft.web.webview2/1.0.3650.58/microsoft.web.webview2.1.0.3650.58.nupkg).
Archive: 8,965,153 bytes, SHA-256
`911a472128c82ac8baa0c486c23342cc9dd6e7dc50d754e676726642ca065c60`.
All three `build/native/x64/` members match the packaged crate candidates exactly:

| Member | Bytes | SHA-256 |
|---|---:|---|
| WebView2Loader.dll | 160,320 | `8427b1fc58ec707813e5c0a51eb5d69397bb333250a7b891be4d3b123f1e0f1c` |
| WebView2Loader.dll.lib | 2,916 | `bfc8ccaaa056be95243a5b66a827e5849d2bb39676fca4dcc2053796d8e15c6d` |
| WebView2LoaderStatic.lib | 10,500,016 | `0659b741bde6348d4c4a6ec4ceb9af50e3d0048ed9cd3c8659bccbb61fde55ee` |

The archive nuspec identifies the same package version and `LICENSE.txt` as
its license source. Original LICENSE and NOTICE text are collected together;
no SDK NOTICE subsection is silently omitted based on unproved linker reach.
[Microsoft distribution documentation](https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/distribution)
distinguishes the loader from the separately installed Runtime. These SDK
materials must not be represented as proof of Runtime redistribution terms.

Files below `legal/third-party/native/Microsoft.Web.WebView2/1.0.3650.58/` are
**transcriptions** written with apply_patch, not byte-identical archived members.
LF/final-newline normalization changes their identities; no original-byte claim.
An exact text comparison verifies all three transcriptions against the original
archive strings after only the declared CRLF→LF plus final-newline conversion.

| Text | Upstream bytes / SHA-256 | Local bytes / SHA-256 |
|---|---|---|
| [LICENSE.txt](../../legal/third-party/native/Microsoft.Web.WebView2/1.0.3650.58/LICENSE.txt) | 1,487 / `0af8f1b807512aae39c2ac1aa4d0cae65cabecb6fd554b8439a5162a0d6eca55` | 1,488 / `324083bf015092054be9d5851dafa1b79cbfab3331ff983476d502839515d8bb` |
| [NOTICE.txt](../../legal/third-party/native/Microsoft.Web.WebView2/1.0.3650.58/NOTICE.txt) | 3,894 / `106423785c5b7eba0a8e61d1837f2132e9c828e20ad530f565d981c1df60dd90` | 3,895 / `b75b4332450ff7b8dab78770247022b89f58cdde358a5d2afbd9e246d6e01caf` |
| [Microsoft.Web.WebView2.nuspec](../../legal/third-party/native/Microsoft.Web.WebView2/1.0.3650.58/Microsoft.Web.WebView2.nuspec) | 1,367 / `7cef90215900c89711502ae3aae1fc31bca3c3c736be2264d2d3cfa8b537aaf2` | 1,347 / `9c6630cc19a0bc575f91af41e441aada7d6abe9e0a0e1060cf35afcf6cf1179c` |

## Review-snapshot mismatch, not silently repaired

Fresh read-only `captureReviewInputs` matches four lock/manifest/frontend
hashes and runtime Cargo tree hash; all 232 candidate identities remain.
Metadata SHA changes from draft
`5f1adebf28b609f452893de3f163c5d26b9297faf0f8ed6bcacc2d3ca5e5e8f4`
to current `ef28a2985a3cea826d9772780672944fe38a73001e96752d20f2027c03e7c2c8`.
The input-check call and three SDK transcription writes shared one orchestration
call; writes occurred before root saw the mismatch. Counsel was then consulted
before further diagnosis/edits. Counsel allows retaining independently useful
pending source evidence, not promoting review status or silently refreshing draft.

Bounded read-only diagnosis compares what the draft actually retains:
213/213 Cargo identities, license declaration/repository/homepage/source fields,
and all 636 recorded source-manifest/VCS hashes match; no added runtime package.
Old raw metadata was not found in the owned QA/perf evidence directories.
The original complete metadata is therefore unavailable for structural diff.
Added private binary targets are a hypothesis only; cause remains unproved.
Runtime package/source correspondence does **not** reconcile full metadata bytes.

Draft SHA stays `3703e5681304948b4c449a7d0b2438c502a2d575930f94a6a741535dd128c95d`;
actual release-manifest SHA stays
`0d6e4322176f59281625f7c382ffcff5b867a05ed674e7f6c8369efcc84cc4f6`.
Both remain pending; neither is overwritten. A future current review snapshot
requires a separate explicit reconciliation/regeneration step and actual review.

The later [wrapper/frontend full-text collection](qa-261005-missing-license-source-collection.md)
resolves the missing-source-text step, not preserved draft flags or approval.
Unresolved questions: raw metadata correspondence,
selected terms/NOTICE/source obligations, final linked and installed notice
coverage. No legal approval, acceptance checkbox or release gate closes here.
