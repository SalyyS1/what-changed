# Hosted wiki verification — 2026-10-27

Status: the P23 hosted-projection identification criterion passes. Public v1
remains **NO-GO**. This record is a read-only observation, not a publication.

## Evidence

- GitHub metadata identifies the public repository as `SalyyS1/what-changed`
  and Pages as a workflow deployment at the [source-preview wiki](https://salyys1.github.io/what-changed/).
- The index, `app.js` and `styles.css` returned HTTP 200. Hosted `app.js`
  exactly matched repository `wiki-site/app.js` at
  [commit `9f8c0df94118287d989b1b253f619050aa212543`](https://github.com/SalyyS1/what-changed/commit/9f8c0df94118287d989b1b253f619050aa212543).
- [Deploy wiki site run 40400873909](https://github.com/SalyyS1/what-changed/actions/runs/40400873909)
  completed successfully. Its head commit matched the inspected source.
- The site labels itself “PUBLIC PREVIEW” and says “Source preview. Release
  publication is still gated.” Its source links point to repository documents.
  The inspected index and script contain no binary-download call to action.
- The local [wiki index](../../docs/wiki-index.md) and
  [deployment guide](../../docs/deployment-guide.md#public-source-preview-wiki)
  explicitly identify the hosted wiki as a projection of canonical source docs.

## Scope and validation

The existing deployment guide owns the operational lookup route. This check
closes only [P23's hosted-projection identification criterion](../260829-0437-what-changed-desktop-mvp/phase-23-wiki-and-documentation-completion.md).
It does not establish complete hosted-link/content reconciliation, alignment
with the repaired release candidate, screenshot provenance, installed acceptance
or launch approval. No remote content, release, tag or community post changed.

`node scripts/check-docs.mjs` passed the documentation route and relative-link
checks. Scoped `git diff --check` passed for the edited tracked documents;
the new report also passed a whitespace check against an empty file.

Unresolved questions: none.
