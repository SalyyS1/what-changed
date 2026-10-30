# Current-source installer candidate

2026-10-27: local QA candidate, unsigned, dirty worktree; publication not approved.
The owner [approved exact source notice terms](approval-261027-0059-notice-packet.md).
This record follows the [binary-discovery repair](fix-261027-0059-tauri-binary-discovery.md).

Normal `bun run tauri build` completed, exit 0: Rust release 8m44s, then NSIS.
Frontend inventory and all approved dependency/source input hashes remain exact.
Source snapshot covers 607 build input files, SHA-256
`c103ce8e0b47eeabaf939e16f1b376e7905e035f5ff601fd549713d1e540df00`.
Private snapshot/build logs are under `.qa/installer-*-20261027-run2*`.

| Artifact | Bytes | SHA-256 |
|---|---:|---|
| Raw `What Changed_0.1.0_x64-setup.exe` | 4,525,180 | `379a5739e754af4d2189077592c65a3b0977368821756f91fde094c9d1154da3` |
| `dist-installer/WhatChanged-Setup-x64.exe` | 4,525,180 | `379a5739e754af4d2189077592c65a3b0977368821756f91fde094c9d1154da3` |
| Built `WhatChanged.exe` | 15,389,696 | `7a05b560f6a9b5e6df4158ab2211a851c67014c0eb07e4f951fa15c4c831e0c5` |
| `THIRD_PARTY_NOTICES.txt` | 453,903 | `aea1d355d6864858959b2a9e4c8ee628acd56718d1e6a8b2620961f344f8c6a5` |

Raw artifact and executable live under
`.qa/collector-performance-release-20261015/release/`; raw installer is in
`bundle/nsis/`. The generated NSIS script SHA-256 is
`cb22e88ba2dc6cc40ae2ae0416a57e2cb4e2fc57050d9f0cae9774da0aab0e4c`.
`dist-installer/SHA256SUMS.txt` binds raw, stable-name copy and notices.

Checks passed: notice generation/check; source-input freshness; generated NSIS
behavior/brand surface; active root notice copy; installer names, version/product
metadata, size budget and byte-identical copy. The old published rehearsal pair
is preserved in `.qa/installer-before-approved-build-20261027/`.

Related focused verification: 96 notice tests plus one NSIS-copy test; seeder
23 passed/3 intentional ignored; architecture 12 passed; relocated soak module
8 passed. Documentation links, launch-claim check, version agreement and brand
assets pass. Non-Three frontend gzip is 142,076 bytes against 409,600.
Strict release-profile Clippy also passes for the changed library, seeder and
architecture-test targets (`-D warnings`, 2m48s). A subsequent check confirms
all 607 source hashes and both executable/installer hashes remain unchanged.

The actual-artifact release audit passed against this executable and installer
(one test). The two identity/migration rehearsal guards also passed; they use
temporary fixtures and do not establish a real version-over-version install.
Log: `.qa/installer-artifact-audit-20261027.log`.

Native preparation: the private profile-content isolation/recovery helper passed
52 fixture assertions in Windows PowerShell 5.1, including interrupted moves,
partial rollback, collisions, identity replacement, reparse rejection and exact
restoration. These fixture passes do not establish real-profile access or any
installed acceptance result. Existing monitoring/profile remained untouched
during build and artifact verification.

Native preflight attempts (no installed acceptance claimed):

- The known Desktop folder has OneDrive cloud reparse tags. A narrowly scoped
  guard now permits only non-name-surrogate cloud ancestors of the exact
  product shortcut; ordinary junctions, symlinks and shortcut reparses remain
  rejected. Read-only Windows PowerShell 5.1 checks passed.
- Copying an EFS-encrypted installation/shortcut file to the QA directory on D:
  failed before monitoring quit. The terminal attempt and partial backup were
  preserved; the next controller uses a fixed C: installation-backup path.
- The next attempt completed ordinary Quit and verified the offline profile
  backup, then correctly rejected the first child move across physical volumes.
  No original entry moved; exact rollback verification passed and monitoring
  restarted. Codex's Windows filesystem view redirects newly created Roaming
  siblings to its LocalCache on D:, while the original profile remains on C:.
  A separately launched WMI read-only probe verified the same original root and
  all seven top-level file identities on physical C:, with no canonical profile
  in the Codex overlay. The completed failure evidence remains under
  `.qa/native-installed-volume-failure-20261027/`.

An unpackaged physical-filesystem controller route is being checked before the
next attempt. `NoPackage` alone is insufficient: both tool-launched shells and
the WMI probe reported it despite different filesystem views. Physical handles,
volume identities, user/session and desktop context must establish the route.

Unresolved: installed/native acceptance, real upgrade/uninstall, performance and
accessibility gates, four-hour soak, clean-tree full release run and publication.
