# Chromium replacement artifact spikes

Status: DONE_WITH_CONCERNS

**Current candidate: ungoogled Chromium 156.0.8078.47-1.1.** Exact publisher digest verified; full 497-component embedded credits captured; the existing runner passed its focused compatibility spike. Source-package/license closure remains with the research owner/controller. This spike does not approve redistribution, installer publication, installed-app acceptance, or network confinement.

The first candidate, official Chromium snapshot 1654400, passed compatibility but has placeholder credits and is superseded. Its evidence remains below.

## Replacement: ungoogled Chromium 156.0.8078.47-1.1

### Publisher receipt and binary

- [Publisher release](https://github.com/ungoogled-software/ungoogled-chromium-windows/releases/tag/156.0.8078.47-1.1), published `2026-10-25T16:46:25Z`.
- GitHub API asset ID `675571302`; published size and SHA256 independently checked before download.
- [Exact portable archive](https://github.com/ungoogled-software/ungoogled-chromium-windows/releases/download/156.0.8078.47-1.1/ungoogled-chromium_156.0.8078.47-1.1_windows_x64.zip).
- Local archive: `D:/Project/.WhatChage/.qa/chromium-snapshot-1654400/ungoogled-156/ungoogled-chromium_156.0.8078.47-1.1_windows_x64.zip`.
- Archive bytes: `201848261`; SHA256 `e8ed1486a88146bf72bfe3d22e198808560e16803a9a1ef62e6e3fc17c979800`; matches the publisher's GitHub asset digest.
- Runtime directory: `D:/Project/.WhatChage/.qa/chromium-snapshot-1654400/ungoogled-156/extracted/ungoogled-chromium_156.0.8078.47-1.1_windows_x64`.
- Executable: that directory's `chrome.exe`; SHA256 `b211ae4d314dafae9692ffe2a0090b32fc58fe18fd8c841ad12d471615140200`.
- Windows file identity: Chromium; The Chromium Authors; version `156.0.8078.47`.
- Safe extraction reused the same traversal/symlink checks. Complete archive: 75 files; 444,915,353 bytes. No pruning or shared-runtime replacement.
- Before download: D free `2684178432` bytes; after extraction and testing: `2025766912` bytes. Both original snapshot and replacement retained; no unrelated files removed.

Evidence: [publisher JSON](../../.qa/chromium-snapshot-1654400/ungoogled-156/official-github-release.json), [download receipt](../../.qa/chromium-snapshot-1654400/ungoogled-156/archive-receipt.json), [archive inventory](../../.qa/chromium-snapshot-1654400/ungoogled-156/extraction-inventory.json), [all runtime file hashes](../../.qa/chromium-snapshot-1654400/ungoogled-156/extracted-file-hashes.json).

### Exact credits, terms, and components

Captured using the exact replacement executable with the same deny-all HTTP/HTTPS capture settings described below. No Google executable terms copied. No external notice links followed.

| Internal page | Full text bytes | Result |
|---|---:|---|
| `chrome://credits/` | 5224021 | 497 product entries and 497 license blocks; complete DOM also saved. |
| `chrome://terms/` | 93 | Intentionally blank Chromium terms; no redirect and no external links. |
| `chrome://version/` | 4544 | Confirms version, executable, own temporary profile, and launch arguments. |

- Credits text SHA256: `9c48aad6914260957008db581aa676e1d39d584d15863b80d9fb17f621f644d5`.
- Credits HTML: 5,350,295 bytes; SHA256 `db501f47d0d1edf716f1293b9ea12f3802929f005b3ed1c286e4e80909f0e1a9`.
- Credits include ffmpeg, DirectX-Shader-Compiler, OpenH264, SwiftShader, Chromium Project, ungoogled-chromium, BoringSSL, and Vulkan components. LGPL/GPL license text is present; license/source-route closure is not inferred solely from the credits count.
- No Widevine-named file, `ffmpeg.dll`, `dxil.dll`, standalone license/credits/terms filename, or `license.pak` in the archive. Absence of an FFmpeg DLL does not settle static-link obligations.
- `chrome.dll`: 303,974,912 bytes, SHA256 `f6eafdd30e2633b06f628a0577c189b9be47a4938218aecf55bfad1040af73f3`.
- `d3dcompiler_47.dll`: 4,753,904 bytes, SHA256 `3e56c1a0867aa04cbca948b3664fcac5d51275865cd044a45a3fd6ce636ee914`.
- `dxcompiler.dll`: 26,010,112 bytes, SHA256 `ea756b8fb9fbfd44dcf29acf29e8d2b1a78c421de5c429d2615d9ac97c3f2143`.

Evidence: [component summary](../../.qa/chromium-snapshot-1654400/ungoogled-156/component-summary.json), [full notice receipt with links](../../.qa/chromium-snapshot-1654400/ungoogled-156/notice-capture-evidence.json), [full credits text](../../.qa/chromium-snapshot-1654400/ungoogled-156/chromium-credits-full-text.txt), [full credits HTML](../../.qa/chromium-snapshot-1654400/ungoogled-156/chromium-credits-full-page.html), [terms text](../../.qa/chromium-snapshot-1654400/ungoogled-156/chromium-terms-full-text.txt), [version text](../../.qa/chromium-snapshot-1654400/ungoogled-156/chromium-version-full-text.txt).

### Focused runner compatibility and cleanup

Reused the same actual runner API spike, parameterizing only QA artifact root and executable path. Packaged Node `v24.12.0`; Playwright `1.62.1`; actual `openSession` with the documented headless override. No implementation edits.

- Eight actions and nine assertions passed: navigate, fill, click, read-back, denied other role, cleanup; visible, text equality, field equality, denied-field absence.
- Staff/customer contexts had distinct cookies; staff persistence matched; other role denied; cleanup persisted cancellation.
- Actual authority explicit screenshot passed: 29,227 bytes; SHA256 `82d65c74edfa48eadcc19a134415856276d87478be946ef78802eb3f6d9f2322`.
- Normal close and abort close passed. No browser/fixture/proxy processes or temporary profiles remained.
- Final ports checked clear: fixture `1425`, proxies `61768`, `62753`, `61762`.
- User's What Changed PID `17328` remained present and untouched.
- Same nonfatal 11-close-listener Socket warning observed; no failed/blocked assertion and no suppression.

Evidence: [compatibility JSON](../../.qa/chromium-snapshot-1654400/ungoogled-156/compatibility-evidence.json), [console](../../.qa/chromium-snapshot-1654400/ungoogled-156/compatibility-console.txt), [recipe](../../.qa/chromium-snapshot-1654400/ungoogled-156/synthetic-recipe.json), [screenshot](../../.qa/chromium-snapshot-1654400/ungoogled-156/synthetic-staff-screenshot.png), [cleanup receipt](../../.qa/chromium-snapshot-1654400/ungoogled-156/process-cleanup-evidence.json).

### Windows OS compiler fallback check

The ungoogled archive's `d3dcompiler_47.dll` was temporarily moved to an owned backup outside the browser directory. No source or `runner/bundle` files were changed. With the browser-side copy absent, the same eight-action/nine-assertion recipe, screenshot, normal close and abort-close checks passed. The copied DLL was restored in a `finally` path and its original SHA256 verified.

- Browser-side original: 4,753,904 bytes, SHA256 `3e56c1a0867aa04cbca948b3664fcac5d51275865cd044a45a3fd6ce636ee914`.
- OS fallback: `C:\Windows\System32\D3DCompiler_47.dll`, 4,669,440 bytes, version `10.0.26100.8655`, SHA256 `2a8bc296c24647e0f5b6bb269e1cd2a0d77256a219ba9fafc6688e968532fc3b`.
- Host: Windows 11 Home Single Language, build `26200`.
- Owned GPU process module inspection observed `C:\WINDOWS\SYSTEM32\D3DCompiler_47.dll` loaded, plus the browser-side `dxcompiler.dll`; this is evidence for this host/runtime path only, not a Windows 10 guarantee.
- Runner bundle inventory SHA256 was unchanged before and after: `5c1ddc24f403f822afbfae9218cac230527f1b070a68e971aadb0395848e84a8`.
- No owned processes or fixture/proxy listeners remained after restoration.

Evidence: [fallback receipt](../../.qa/chromium-snapshot-1654400/ungoogled-156/os-compiler-fallback/os-compiler-fallback-receipt.json), [module inspection](../../.qa/chromium-snapshot-1654400/ungoogled-156/os-compiler-fallback/owned-browser-module-inspection.json), [fallback console](../../.qa/chromium-snapshot-1654400/ungoogled-156/os-compiler-fallback/compatibility-console.txt), [reversible test script](../../.qa/chromium-snapshot-1654400/ungoogled-156/test-os-compiler-fallback.ps1).

### Source route handoff

Research owner supplied exact source route pins; this task did not download the 2 GB source archive or certify the closure:

- Official Chromium source lite archive `chromium-156.0.8078.47-lite.tar.xz`, 2,009,127,748 bytes, publisher SHA256 `28432028890ef70e1f2e9db880c500907ad1b04f5ae6b655a3f394b9f2eaa705`.
- Windows platform source commit `258d663c6ee6dd3d4cfe2e05eb8e863627e3a5af`; core source commit `0673f8701c94e5688c33ef8ce5e36602d311d246`.
- Controller plans hosted source staging and its own hash receipts. Final component/source-route decision belongs to that combined evidence.

## Superseded: official Chromium snapshot 1654400

## Artifact and source receipt

- QA root: `D:/Project/.WhatChage/.qa/chromium-snapshot-1654400`
- Official metadata: <https://www.googleapis.com/storage/v1/b/chromium-browser-snapshots/o/Win_x64%2F1654400%2Fchrome-win.zip>
- Download: <https://storage.googleapis.com/chromium-browser-snapshots/Win_x64/1654400/chrome-win.zip?generation=1782775688693704>
- Local archive: `D:/Project/.WhatChage/.qa/chromium-snapshot-1654400/chrome-win.zip`
- Generation: `1782775688693704`; object published `2026-06-29T23:28:08.760Z`.
- Archive bytes: `340981417`.
- SHA256: `ea72622764ec197569ad418a92678430724211cd31cabea07dddeca50cbf74d9`.
- Published MD5 base64 matched: `wjH8Eb0HnRtYMUyBpB1ZDw==`.
- Published CRC32C base64 matched: `xjra7g==`.
- Generation-pinned response generation matched the live metadata. Receipt records complete public response headers.
- Official `REVISIONS` identifies Chromium source `59e897bb7c08867380065818882e6c7d2de599ef`, position `refs/heads/main@{#1654400}`; V8 `b231b8f07c377ee257f573eb690850d24830dd0d`, Dawn `17a1629450913c56b750a8b3edd32f8844f16b4f`, ANGLE `e0580dcb02d82ff5ff3f0aa4cf3c19fabbd0c57a`, SwiftShader `5b0479bd2d15058aaa9eb490e364f920ff824a8c`.

Receipts: [archive](../../.qa/chromium-snapshot-1654400/archive-receipt.json), [live metadata](../../.qa/chromium-snapshot-1654400/official-object-metadata.json), [official revisions](../../.qa/chromium-snapshot-1654400/official-REVISIONS.json), [exact source receipts](../../.qa/chromium-snapshot-1654400/exact-source-receipts.json).

## Extracted identity and size

Extraction rejected absolute paths, parent traversal, backslashes, and ZIP symlinks; destinations verified within this QA root. No shared runtime was moved or replaced.

- Browser directory: `D:/Project/.WhatChage/.qa/chromium-snapshot-1654400/extracted/chrome-win`
- Executable: `D:/Project/.WhatChage/.qa/chromium-snapshot-1654400/extracted/chrome-win/chrome.exe`
- Windows identity: Chromium; The Chromium Authors; version `151.0.7922.0`.
- Executable SHA256: `e6fe7fd621165d7835aeaf90acc1e8773c586d617c986a9583d2aa419501883c`.
- Complete extracted archive: 261 files; 810,425,568 bytes. Post-execution inventory found no added installation files.
- No `ffmpeg.dll` entry anywhere in archive. This does not establish absence of statically linked FFmpeg.
- No standalone file with a license, credits, copyright, or terms filename; no `license.pak`.
- Disk D had 2,684,571,648 bytes free after extraction and testing.

Inventory: [archive entries](../../.qa/chromium-snapshot-1654400/extraction-inventory.json), [all extracted SHA256 hashes](../../.qa/chromium-snapshot-1654400/extracted-file-hashes.json).

All top-level archive files:

| File | Bytes |
|---|---:|
| 151.0.7922.0.manifest | 224 |
| chrome_100_percent.pak | 695462 |
| chrome_200_percent.pak | 1197041 |
| chrome_elf.dll | 1489408 |
| chrome_proxy.exe | 1373696 |
| chrome_pwa_launcher.exe | 1661952 |
| chrome_wer.dll | 109056 |
| chrome.dll | 317893120 |
| chrome.exe | 2924032 |
| D3DCompiler_47.dll | 4741488 |
| dxcompiler.dll | 24879104 |
| dxil.dll | 1509760 |
| elevated_tracing_service.exe | 4109824 |
| elevation_service.exe | 2902528 |
| eventlog_provider.dll | 6144 |
| First Run | 8 |
| icudtl.dat | 10876560 |
| interactive_ui_tests.exe | 344262144 |
| libEGL.dll | 473600 |
| libGLESv2.dll | 473600 |
| notification_helper.exe | 1497088 |
| resources.pak | 21614900 |
| setup.exe | 4741632 |
| v8_context_snapshot.bin | 740440 |
| vk_swiftshader_icd.json | 106 |
| vk_swiftshader.dll | 5576704 |
| vulkan-1.dll | 990208 |

The archive also contains locales and other nested resources; the linked inventory is complete. No pruning was performed.

## Compatibility evidence

Executed with the existing packaged `runner/bundle/node.exe` (`v24.12.0`) and `runner/bundle/playwright-core` (`1.62.1`). Used actual current-source `openSession`, `locatorFor`, `validateRecipe`, `executeScenario`, `makePolicy`, `createAuthority`, and existing synthetic booking fixture. No source implementation changed.

The spike wraps `chromium.launch` to set `headless: true`; all other actual runner launch options remain supplied by `openSession`, including `chromiumSandbox: true` and its proxy. This establishes headless API compatibility only. It does not verify visible packaged windows or native parent integration.

All passed:

1. Artifact launch, CDP version, Chromium version-page identity, and own temporary profile path under this QA root.
2. Independent staff/customer browser contexts and distinct fixture cookies.
3. Eight real recipe actions: navigate, two fills, click/save, staff read-back, other-role denial, cleanup, final read-back.
4. Nine assertions covering visible, exact text, fixture field equality, and denied response without protected field. Staff read-back matched persisted synthetic data; customer read-back returned forbidden; cleanup persisted `cancelled`.
5. Actual authority screenshot handling after an explicit synthetic capture frame. PNG: 29,233 bytes; SHA256 `0fa97299547964340f7d9a9d445beb7d60834fbc2dced1f723664593a7d81b89`.
6. Normal session close closed both pages.
7. Abort signal closed a separately opened runner browser; subsequent close released its proxy.

Fixture: `http://127.0.0.1:1425`; synthetic data only. Per-step release was automated by the spike, with cleanup completion observed before final read-back. This does not prove a real user's consent flow.

Evidence: [compatibility JSON](../../.qa/chromium-snapshot-1654400/compatibility-evidence.json), [console](../../.qa/chromium-snapshot-1654400/compatibility-console.txt), [recipe](../../.qa/chromium-snapshot-1654400/synthetic-recipe.json), [screenshot](../../.qa/chromium-snapshot-1654400/synthetic-staff-screenshot.png), [spike script](../../.qa/chromium-snapshot-1654400/compatibility-spike.mjs).

Observed diagnostic limits:

- Chromium rejected the first probe's `Browser.getBrowserCommandLine` because `--enable-automation` was absent. The corrected probe reads `chrome://version`; no extra browser flag was added. Initial diagnostic evidence retained in `compatibility-first-probe-failure.json`.
- Successful console contains `MaxListenersExceededWarning`: 11 close listeners on a Socket. No assertion failed; listener warning remains for controller review. It was not suppressed or repaired in this isolated task.

## Exact artifact notices capture

Captured full `body.textContent`, complete DOM HTML, titles, final URLs, links, lengths, and SHA256 values from the exact executable. During capture all HTTP/HTTPS was denied by the authenticated deny-all proxy, context routes, and host resolver arguments; service workers blocked. No outbound site was intentionally opened. These capture settings are not a machine-wide confinement proof.

| Internal page | Full text bytes | Observation |
|---|---:|---|
| `chrome://credits/` | 177 | Sample credits placeholder, zero component entries and zero license entries; requests `generate_about_credits=true`. |
| `chrome://terms/` | 93 | Intentionally blank Chromium terms resource, no Google terms redirect or external links. |
| `chrome://version/` | 4428 | Chromium version, executable path, isolated profile and launch arguments. |

Captured credits begin: “This is sample credits page.” The full HTML confirms the placeholder; this is not delayed rendering of component notices. No Google executable terms were copied into this artifact.

Evidence: [notice receipt](../../.qa/chromium-snapshot-1654400/notice-capture-evidence.json), [credits text](../../.qa/chromium-snapshot-1654400/chromium-credits-full-text.txt), [credits HTML](../../.qa/chromium-snapshot-1654400/chromium-credits-full-page.html), [terms text](../../.qa/chromium-snapshot-1654400/chromium-terms-full-text.txt), [version text](../../.qa/chromium-snapshot-1654400/chromium-version-full-text.txt), [capture script](../../.qa/chromium-snapshot-1654400/capture-notices.mjs).

The exact source root [LICENSE](https://chromium.googlesource.com/chromium/src/+/59e897bb7c08867380065818882e6c7d2de599ef/LICENSE) permits source/binary redistribution subject to its conditions. A fetched exact-revision copy is retained under `exact-source/LICENSE`, SHA256 `368cca1106be99d39ecd32a38d8305585d802a475effb66380b91ffc9bcf709b`. Exact source VERSION, unbranded BRANDING, and archive GN args retained with URLs/hashes in the source receipt. This root license alone does not close included component obligations.

## Process cleanup

All owned fixture/browser/proxy processes exited. Final census found no Chromium executable or Node command from this QA root, no listeners at fixture port 1425 or measured proxy ports 52312/59522/52313, and no temporary profiles. User's What Changed PID 17328 remained present and untouched.

Evidence: [cleanup receipt](../../.qa/chromium-snapshot-1654400/process-cleanup-evidence.json).

## Unresolved questions

- For the current ungoogled candidate: exact FFmpeg/static-link obligations, DirectX component notices, and the hosted source/relink route remain with the license research owner/controller. Absence of `ffmpeg.dll` does not settle them.
- The superseded official snapshot has incomplete embedded notices and should not be promoted from this spike.
- Visible packaged runner, installer, native permission workflow, and network-boundary acceptance remain outside this isolated spike.
