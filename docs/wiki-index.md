# What Changed documentation

**Evidence state: available index; each page distinguishes verified evidence, known limitations and planned work.**

This repository is the canonical, offline-readable documentation source. Each
page labels the evidence state of a claim:

- **Available** — implemented in the current tree.
- **Verified here** — exercised by a named test or local measurement.
- **Known limitation** — deliberately bounded or unavailable on this machine.
- **Planned** — accepted scope that is not in the release candidate.

## Use

1. [Getting started](getting-started.md)
2. [First investigation](first-investigation.md)
3. [Timeline and search](timeline-and-search.md)
4. [Checkpoints and maps](checkpoints-and-maps.md)
5. [Offline owner workflow — development source](ownvero/offline-owner-workflow.md)
6. [Ownvero quickstart](ownvero/getting-started.md)
7. [Rules, requests and project memory](ownvero/owner-workspace.md)
8. [Preview verification and limits](ownvero/verification.md)
9. [2D/3D maps and keyboard list](ownvero/maps.md)
10. [Ownvero FAQ](ownvero/faq.md)

The [live English landing/wiki](https://salyys1.github.io/what-changed/) and
[Vietnamese version](https://salyys1.github.io/what-changed/vi/) explain the owner
workflow with an interactive 3D model, a 2D alternative and local search.
This is a website preview; the Windows download remains unavailable pending
exact release acceptance. The [site source and local commands](../wiki-site/README.md)
remain available offline.

## Understand and trust

- [Product scope](../PRODUCT.md)
- [Evidence and analysis rules](../ANALYSIS.md)
- [Settings and privacy](settings-and-privacy.md)
- [Diagnostics and troubleshooting](diagnostics-and-troubleshooting.md)
- [Known limitations](known-limitations.md)
- [FAQ](faq.md)
- [Accessibility audit](accessibility-audit.md)
- [Performance baseline](performance-baseline.md)
- [Brand guide](brand-guide.md)

## Contribute and release

- [Architecture](../ARCHITECTURE.md)
- [Collector boundary](../COLLECTORS.md)
- [Release verification](release-audit.md)
- [Installed-app acceptance walkthrough](acceptance-walkthrough.md)
- [Public-v1 go/no-go packet](public-v1-go-no-go.md)
- [Deployment guide](deployment-guide.md)
- [Development preview packaging](development-preview.md)
- [Historical public-preview navigation](wiki-public-preview.md)
- [Release checklist](release-checklist.md)
- [Owner repeat-use pilot protocol](ownvero/pilot-protocol.md)
- [Third-party notices and audit gaps](../THIRD_PARTY_NOTICES.md)
- [Support](../SUPPORT.md)
- [Support policy](support-policy.md)
- [Security reporting](../SECURITY.md)

The current public-v1 packet is **NO-GO**. The [installed candidate evidence](../plans/reports/qa-261027-0434-mvp-continuation.md)
records partial native acceptance and a workspace investigation failure; that candidate predates
the subsequent source repair. Source checks and phase tests do not close its
installed-app, upgrade, soak, accessibility, full 1M performance or
human-approval rows. Those rows need evidence for one named candidate before a
binary release or launch copy can be approved.

The [launch package](launch/messaging.md) is public-facing copy and links
back here for technical truth. The authorized [public source-preview wiki](deployment-guide.md#public-source-preview-wiki)
is a projection of the source docs, not a binary release approval.
