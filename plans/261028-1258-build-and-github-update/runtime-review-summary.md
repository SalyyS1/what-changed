# Runtime review packet for the requested complete preview

Status: exact packet prepared; current browser redistribution is not authorized
by the reviewed public terms. No runtime approval or release inferred.

## Exact scope

- Node v24.12.0, Playwright 1.62.1, browser revision 1234 / Chrome for Testing 151.0.7922.34.
- Runtime: 433 files / 550836112 bytes. Inventory SHA256
  `ec77cefc82ae2b433b188391c36452a4302c049f6778228e53869527b3c66b1b`.
- [Proposed record](proposed-runtime-review.json), [source hashes](runtime-sources.json).
- [Full notices](../../release/RUNNER_THIRD_PARTY_NOTICES.txt).
- Terms were extracted from the exact packaged browser with HTTP(S) requests blocked.
  Full collapsed credit texts were captured, not only visible component headings.

## Retained terms

Node's complete release license includes its bundled dependency terms. Playwright's
Apache license, NOTICE and all three packaged sidecar license files are retained.
Chrome's complete embedded credit texts and executable terms are retained.

The packaged browser identifies itself as Google Chrome for Testing. Its executable
terms refer to Google's terms and AVC restrictions; it is not classified here as
solely BSD Chromium. The Chromium source license alone does not establish permission
to redistribute this exact executable. The owner requested verification of the
current browser before considering another package.

The [official-source check](../reports/research-261028-1638-browser-redistribution.md)
found that CfT uses Chrome executable terms and that Google's public software
terms prohibit distribution. No separate applicable grant was supplied or found.
Owner acceptance of notices cannot grant Google's rights. Bundling this browser
for distribution remains blocked; a separately applicable grant or a different
browser artifact with verified redistribution rights is needed.

Sources: [Node release license](https://raw.githubusercontent.com/nodejs/node/v24.12.0/LICENSE),
[Playwright source](https://github.com/microsoft/playwright/tree/v1.62.1),
[Chrome executable terms](https://www.google.com/chrome/terms/),
[Google terms](https://policies.google.com/terms).

## Decision boundary

The existing `check-runner-notices` script requires a named human review bound to
this exact inventory and full notice payload. The prior approval covers desktop
dependencies only. Creating source/build infrastructure is authorized; recording
that the owner reviewed new terms cannot be inferred from a cleanup/build request.

If applicable redistribution rights are established, exact human notice review
would permit installer QA with that runtime and retained notices.
Public publication still requires an artifact-specific decision after a successful
build, with unsigned status and unverified acceptance boundaries disclosed.

Unresolved questions: separate permission for the current executable or owner
decision to select a browser with verified redistribution rights; exact notice
review after that decision.
