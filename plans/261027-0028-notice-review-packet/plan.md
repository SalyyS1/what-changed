---
title: "Current-source third-party notice review packet"
status: completed
parent: "../260829-0437-what-changed-desktop-mvp/phase-21-installer-updater-release.md"
---

Outcome: deliver a complete, frozen source-review proposal so the existing human
license/obligation review can precede the current installer build. Consumers:
the user/reviewer and the release workflow. This is execution evidence, not a
new legal policy or publication approval.

Constraints: retain the old draft and pending release manifest; never fabricate
a reviewer or approval; no installer bypass; original monitoring/profile remain
in place. Raw machine-specific Cargo metadata stays in ignored `.qa/` storage.
The old unavailable metadata cannot be structurally reconciled; a fresh snapshot
must explicitly supersede it, preserving that limitation.

- [x] Capture and bind current raw inputs; compare candidate/source identities.
- [x] Recommend terms, full text, NOTICE and source routes for every candidate.
- [x] Include independently sourced SQLite and WebView2 SDK native proposals.
- [x] Validate coverage, hashes, provenance and unapproved state; independent review.
- [x] Present one readable review packet and the exact remaining human decision.

Completed: 232 packages plus two native components; 263 referenced texts; 96
existing tests passed; independent technical and final checksum review passed.
The user then [approved this exact packet](../reports/approval-261027-0059-notice-packet.md)
for notices/build/QA on 2026-10-27. The frozen source proposal stays unchanged;
approval promotion and installer validation continue under the parent phase.

Acceptance: every current input accounted for; all referenced text/hash identities
verified; source exceptions explained; preview conspicuously pending; release
approval fields untouched. No public installer or publication claim follows.

Context: [previous source collection](../reports/qa-261005-missing-license-source-collection.md),
[native correspondence](../reports/qa-261005-native-notice-source-correspondence.md),
[existing preparation owner](../../scripts/prepare-third-party-review.mjs).

Rollback: packet is additive; deleting its generated proposal has no runtime or
release effect. Preserve ignored input snapshots while review is pending.
