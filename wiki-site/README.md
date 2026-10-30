# Ownvero local source preview

Static landing/wiki/demo for public review of the owner-workspace direction.
Identity remains a G2 draft; binary publication remains G3-gated. The output has
no desktop IPC and does not expose a download until an exact release is approved.
The opening exhibit follows Intent → Preserve → Brief → Verify → Decide through
one synthetic booking change. Its lit 3D workbench opens after first paint with
rotation and a 2D toggle; the static map and complete walkthrough remain available
without JavaScript or WebGL. The full demo route keeps 3D opt-in. See the shared
[renderer](src/workbench-3d.mjs) and [walkthrough](src/workflow-exhibit.mjs).

## Build and review

From the repository root, after its dependencies are installed:

```powershell
node wiki-site/build.mjs
node --test wiki-site/tests/*.test.mjs
node scripts/check-site.mjs
node scripts/check-download-manifest.mjs
node wiki-site/serve.mjs
```

The server uses 127.0.0.1:5184 and the preserved /what-changed/ base path. Check
that port’s owner before starting; reuse a server only if it serves this tree.
Stop the server after review. The build writes only wiki-site/dist and rejects
unexpected output, private paths, secret signatures, and desktop bridge code.

The browser suite starts its own server only when 5184 is free, tracks the child,
and stops it when finished. It uses the installed Playwright browser or Edge.
The reviewed Markdown allowlist lives in site.config.mjs; raw HTML is escaped.

## Source reconciliation

The remote source was inspected read-only at the pinned upstream commit in
site.config.mjs. Its three-file wiki had no generator or framework. This build
keeps the static route and replaces browser-only content with pre-rendered HTML.
legacy-aliases.json maps the seven original hash links; their anchors also remain
on the landing for no-JavaScript reading. The build never copies plans, QA output,
desktop bundles, local screenshots, or an entire source directory into the site.

The GitHub Actions workflow builds the allowlisted output and deploys this clearly
labeled review preview to GitHub Pages. It does not publish an installer or enable
the download route. Do not upload wiki-site itself: the explicit generated output
is wiki-site/dist.

## Download authority

The [schema](../release/release-manifest.schema.json) and
[validator](../scripts/check-download-manifest.mjs) own the strict format. Missing,
invalid, or unapproved metadata disables the link. Enabling a release additionally
checks the approval ledger and requires G3 to bind this exact manifest. GitHub
release tag, artifact filename, source commit, notices, byte count, and SHA-256 are
explicit. No latest redirect or local QA path is accepted.

Development docs show their content revision and review date. An approved
manifest binds two separate assets from the same GitHub Release tag: the Windows
installer (`assetUrl`, `bytes`, `sha256`) and the corresponding source archive
(`sourceAssetUrl`, `sourceBytes`, `sourceSha256`). The build manifest records
source commit and documentation hashes; it is local preview provenance, not a
signed release or proof of installed-app acceptance. The checked-in manifest is
still intentionally unavailable until G2/G3 approval.
