# Third-party notices

**Evidence state: exact-source notice review approved on 2026-10-27; notices generated and checked. Frozen installer/installed coverage remains open; publication is not approved.**

The application is licensed under the [MIT License](LICENSE). That license does not replace the licenses and copyright notices of third-party code. The version authorities are [Cargo.lock](src-tauri/Cargo.lock) and [bun.lock](bun.lock), not a hand-maintained package list here. A lockfile identifies candidates; it does not prove which code is present in the installer.

## Audit snapshot — 2026-09-29

This inspection used `cargo metadata --locked --offline` filtered to `x86_64-pc-windows-msvc`, local Cargo package manifests and their top-level license files, and the exact-version packages installed from `bun.lock` under `node_modules`. The Cargo graph includes build/proc-macro dependencies as well as application dependencies; the Bun install includes developer tooling. Neither is an artifact-level shipped-code inventory.

| Lockfile | SHA-256 at inspection |
|---|---|
| `src-tauri/Cargo.lock` | `E00218729E5638649FCFFB75A14A70D43BE6AAB9F85FF001ABAD82D8B1B97CD6` |
| `bun.lock` | `00255794C32424FDD0E5E814A517995F0AE0A0129F2AC82A928004918D0A049E` |

Every package in the resolved Windows Cargo metadata declares a license. The declarations include permissive licenses, Unicode-3.0 and MPL-2.0; the latter cannot be silently treated as the application's MIT license. The following third-party Cargo packages have **no top-level license text** in this machine's registry extraction (not a claim that upstream has no license):

| Declared license | Package versions missing a local top-level text |
|---|---|
| BSD-3-Clause | `alloc-stdlib 0.2.4` |
| Apache-2.0 | `rusqlite_migration 2.6.0` |
| MPL-2.0 | `selectors 0.36.1` |
| MIT | `ts-rs 12.0.1`, `ts-rs-macros 12.0.1`, `webview2-com 0.38.2`, `webview2-com-macros 0.8.1`, `webview2-com-sys 0.38.2` |
| `MIT OR Apache-2.0` / `Apache-2.0 OR MIT` | `defmt-parser 1.0.0`, `tauri-plugin 2.6.3` |
| `MIT/Apache-2.0` (legacy slash form) | `unic-char-property 0.9.0`, `unic-char-range 0.9.0`, `unic-common 0.9.0`, `unic-ucd-ident 0.9.0`, `unic-ucd-version 0.9.0` |

The installed Bun packages have these metadata/text exceptions: `svelte-toolbelt 0.10.6` omits a `license` field in `package.json`, although its bundled `LICENSE` is MIT; `@tauri-apps/plugin-dialog 2.7.3` has a `LICENSE.spdx` package declaration but no full license text in that package. The installed packages `@rolldown/binding-win32-x64-msvc 1.2.7`, `@tauri-apps/cli-win32-x64-msvc 2.11.4`, `is-reference 3.0.3`, `locate-character 3.0.0`, `saxes 6.0.0`, and `stackback 0.0.2` have no top-level license text. Installed Bun metadata also declares MPL-2.0 for `lightningcss` and its Windows binding, and BlueOak-1.0.0 for `lru-cache`; their shipped status is not established. Some `bun.lock` entries are not installed on this Windows machine, notably optional binaries for other platforms and nested resolution entries, so their texts were not verified locally. These findings do not establish that every listed package ships in the application.

The application `LICENSE` in [Tauri bundle configuration](src-tauri/tauri.conf.json) covers only What Changed's own code. This file itself contains no third-party license texts.

## Fail-closed release scaffold

The release workflow invokes [the notice generator](scripts/generate-third-party-notices.mjs) before an installer can be built. Its versioned [review manifest](scripts/third-party-notices-manifest.json) now records the project owner's [explicit approval](plans/reports/approval-261027-0059-notice-packet.md) of 232 package entries and two native components. Generation and exact-payload checks passed. This source-review approval permits installer QA; it does not approve publication or replace the remaining release gates. A `-SkipInstaller` local rehearsal still records the notice gate as skipped, not passed.

The final Vite build records its dependency module inputs and package CSS imports (including Tailwind CSS before transformation) outside `dist`; the generator also resolves the locked Windows release-target Rust graph without proc-macro/build edges. These are conservative notice candidates, **not proof that every package was retained in the installer**. A reviewer must approve an entry for each candidate, including its exact version, declared and selected license, authoritative HTTPS source, obligation/NOTICE-file decision and hashed local full text under `legal/third-party/`. Selected MPL/GPL-family terms also require a source-availability URL. The generator rejects missing/extra entries, stale lockfile hashes, unresolved frontend modules and modified texts. Ordinary Cargo builds create an ignored `PENDING - NOT FOR RELEASE` resource placeholder if no notice exists, because Tauri validates resource paths during compilation. Only a fully reviewed manifest can replace it with a publishable `dist-installer/THIRD_PARTY_NOTICES.txt`; the installer verifier rejects the placeholder and expects an active copy of the reviewed bytes in the generated NSIS script and checksum manifest. Bypassing Tauri's before-build command can still create an unverified raw bundle, which must not be distributed. Installed-file inspection remains a separate release-candidate check.

The [approved source packet](plans/261027-0028-notice-review-packet/review-summary.md) resolves the runtime candidate text/NOTICE choices, independent SQLite and WebView2 SDK terms/provenance, supplemental attributions and unmodified MPL source availability. The earlier broad inventory also names build-only candidates and is not a shipped-package list. Do not change review status merely to make a build pass; the recorded approval is bound to the exact frozen packet, and changed inputs need correspondence checks and any newly required review.

The fail-closed source contract is separate from wrapper metadata: known SQLite and WebView2 wrappers require independently reviewed native records with original license/NOTICE bytes and SHA-256s, exact contained source/native/selection byte evidence, a `src-tauri/Cargo.toml` SHA binding, and usable HTTPS source, source-availability and provenance routes. A nullable native version is valid only with explicit provenance review. The generator applies this contract only after the pending-manifest guard, then resolves locked offline metadata; this is a prebuild candidate-freshness check, not proof of native linker, executable, installer contents or final legal approval.

## Before public distribution

The [2026-10-05 source collection](plans/reports/qa-261005-missing-license-source-collection.md)
adds complete, explicitly transcribed upstream texts for the eleven missing-text
runtime candidates; [independent native correspondence](plans/reports/qa-261005-native-notice-source-correspondence.md)
also identifies the exact SQLite source and Microsoft SDK loader candidates.
Those records preserve upstream and local hashes separately. The later
[2026-10-27 packet](plans/261027-0028-notice-review-packet/review-summary.md)
captures fresh raw inputs, verifies package/source correspondence, and explicitly
supersedes the old snapshot without claiming to explain its unavailable raw
metadata. The old draft is preserved. The separate approval record authorizes
terms and obligations for the new source packet, not a shipped artifact.

- Freeze the exact installer, source revision and lockfiles; determine which Rust code and browser-bundle packages are actually distributed, separating build/test-only tools.
- Retain the approved chosen terms, copyright/NOTICE texts and source-availability route with the release. Recheck any candidate or source changes against the approved packet.
- Include or attach the resulting notice payload with that release and verify it against the frozen artifact. Repeat the audit after either lockfile changes.

Until then, the third-party notice obligation in the [public-v1 readiness plan](plans/260829-0437-what-changed-desktop-mvp/phase-25-public-v1-readiness-and-launch.md) remains open.
