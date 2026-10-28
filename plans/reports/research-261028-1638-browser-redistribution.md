# Chrome for Testing redistribution check

Research completed: 2026-10-28 16:44 ICT / 09:44 UTC.

## Result

**NO under the reviewed public terms.** The official sources do not grant permission to embed this exact full Chrome for Testing executable in another desktop application's Windows installer. Google's applicable public software terms expressly prohibit distribution. Permission through a separate agreement remains **UNRESOLVED**; none was supplied or found. This is a conclusion about the reviewed license evidence, not a determination of every possible statutory right. [Google software terms](https://policies.google.com/terms?hl=en&gl=vn), [CfT maintainer clarification](https://github.com/GoogleChromeLabs/chrome-for-testing/issues/21).

Owner acceptance of notices cannot supply rights owned by Google. Retaining credits and executable terms remains necessary evidence, but does not create a redistribution grant.

## Exact scope and local evidence

- Requested delivery: Windows installer containing Node and the browser; no first-launch browser download.
- Reviewed runtime: Node 24.12.0, Playwright 1.62.1, browser revision 1234, **Google Chrome for Testing 151.0.7922.34**, obtained through Playwright's CDN.
- Existing inventory: 433 files / 550836112 bytes; inventory SHA256 `ec77cefc82ae2b433b188391c36452a4302c049f6778228e53869527b3c66b1b`.
- Local evidence read: [runtime review summary](../261028-1258-build-and-github-update/runtime-review-summary.md), [exact browser executable terms](../../legal/third-party/runner/chrome-151.0.7922.34/CHROME-TERMS.txt).
- This check addresses the full CfT browser's redistribution grant. It does not re-audit Node's license, all bundled third-party components, or the installer.

## Official evidence and applicability

| Source and date | Evidence | Application to this bundle |
|---|---|---|
| [CfT issue #21](https://github.com/GoogleChromeLabs/chrome-for-testing/issues/21), GoogleChromeLabs member Mathias Bynens, 2023-06-16; checked 2026-10-28 | Maintainer states the repository license covers repository code. He describes CfT as “a flavor of Google Chrome” and points to Chrome's terms for the binary. | Direct answer distinguishing the dashboard/API repository from the CfT executable. The issue author's later conclusion is not a Google licensing statement and is not relied on. |
| [Chrome additional terms](https://www.google.com/chrome/terms/), modified 2025-09-30; checked 2026-10-28 | Terms incorporate Google Terms of Service and “apply to the executable code version of Chrome and ChromeOS”. They separately identify most Chrome source code as available under open-source agreements. | Matches the retained embedded terms for 151.0.7922.34. The live heading additionally mentions GooglebookOS; the relevant executable/source distinction and modification date agree. No installer redistribution permission appears here. |
| [Google Terms, Vietnam](https://policies.google.com/terms?hl=en&gl=vn) and [United States](https://policies.google.com/terms), effective 2026-07-30; checked 2026-10-28 | Under **Software in Google services**: “You may not copy, modify, distribute, sell, or lease any part of our services or software.” The use license is personal and non-assignable. Open-source terms may override provisions for software offered under those licenses. | Both country versions contain the same relevant restriction. No reviewed source establishes that an open-source license overrides it for this entire Google-branded executable. Recipient countries and any separate agreement still need exact applicability review. |
| [CfT repository LICENSE](https://github.com/GoogleChromeLabs/chrome-for-testing/blob/main/LICENSE), Apache 2.0; checked 2026-10-28 | Apache rights attach to the work made available under that license; the maintainer limits the repository license to its code. | Licenses dashboard/API tooling. It is not evidence that the separately downloaded Chrome executable is Apache licensed. |
| [Chromium source LICENSE](https://chromium.googlesource.com/chromium/src/+/main/LICENSE), blob `2249a28657f86c645f8978b75babc1bd52aca318`; checked 2026-10-28 | BSD-style license permits source/binary redistribution subject to notice, disclaimer and endorsement conditions. | Establishes rights for the covered Chromium code. This current source file is not a complete license inventory for the packaged CfT binary and is not asserted to be the exact 151.0.7922.34 source snapshot. |
| [Chrome Enterprise Core Agreement](https://chromeenterprise.google/terms/chrome-enterprise-core/), modified 2024-08-29; checked 2026-10-28 | Scope excludes the browser: “This Agreement does not cover use of (a) the Chrome Browser itself”. Browser use is separately governed by Google Terms. | Cloud management terms do not provide a CfT desktop-app bundling grant. Calling management a standalone offering does not change the covered product. |
| [Playwright browser documentation](https://playwright.dev/docs/browsers), current undated page; checked 2026-10-28 | Documents browser downloads from Microsoft's CDN, custom artifact hosting and local browser placement. Its general description calls the default Chromium builds open source. | Operational download/cache documentation contains no license grant for this exact CfT executable. Generic names and CDN availability do not override its embedded identity and executable terms. |

### Enterprise standalone route

The candidate official routes `/chrome/terms/standalone-enterprise/`, `/chrome/terms/standalone-enterprise.html`, and `/chrome/terms/standalone-enterprise/index.html` did not yield readable current terms. A direct HTTPS request to the trailing-slash route returned **404**. No favorable clause is inferred from these unavailable routes. The current [Chrome Enterprise terms index](https://chromeenterprise.google/resources/terms/) and the readable Core agreement do not establish another applicable executable license. The index's other browser agreement, [Chrome Browser Cloud Management](https://chromeenterprise.google/terms/chrome-browser-cloud-management/) (modified 2023-12-13), defines its Services as the cloud management service; Sections 1.9(a), 4.1 and 15 do not grant CfT binary redistribution. Historical or separately negotiated enterprise rights cannot be transferred to this CfT package by assumption.

### Recent issue #226

[Issue #226](https://github.com/GoogleChromeLabs/chrome-for-testing/issues/226) was opened 2026-09-28 and remained open when checked. It asks about **chrome-headless-shell 154.0.8037.57 in a private Linux commercial PDF container**, a different product/version/platform/use case. Its visible 2026-10-14 reply expressly disclaims Google affiliation; no Google licensing answer is visible. It cannot authorize this Windows full-browser installer. Claims in that reply about other archives' contents were not independently checked and are not adopted.

### AVC notice

The retained executable terms also contain an AVC patent notice limiting described licensed uses and rejecting an implied license for other uses. The notice is not a general redistribution grant. This report does not decide the app's codec/patent position; resolving Google's executable distribution restriction would not automatically settle that separate question. [Chrome additional terms](https://www.google.com/chrome/terms/).

## Concrete next step

Do not close the current browser redistribution row as approved on the strength of notices or user acceptance alone. To retain this exact CfT browser, obtain an applicable written grant from Google covering copying and redistribution inside this desktop installer, recipient scope, full packaged components, notices and any relevant media rights. No contact was made and no issue was created during this research.

If no applicable grant is available, the next technical decision is a browser artifact whose publisher expressly grants binary redistribution for the intended installer. Verify its exact executable, component notices and Playwright compatibility before replacing the current package. This preserves the user's requirement for an installer containing the browser and Node; it does not select or implement a browser change.

## Unresolved questions

- Does the owner already hold, or can Google supply, a separate agreement that expressly covers this exact CfT bundling route?
- Which recipient jurisdictions and media uses must that agreement or a replacement artifact cover?
