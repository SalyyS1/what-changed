# Website overhaul delivery evidence

## Verified locally

Windows/Edge headless; 2026-10-30, Asia/Saigon. Site build/check passed with all
19 routes. Full site suite: 12 passed, zero failed/skipped. Exact release remains
unavailable. No desktop profile or personal data was changed by these checks.

The browser suite covers opening 3D, six shared graph nodes, steps/rotation/2D,
lazy full demo, no-JavaScript reading, denied WebGL, actual context loss/retry,
delayed dynamic imports across persisted-page events, retained zoom/layer/
separation controls, Vietnamese model labels, no overflow and no outbound requests.
Persisted-page events are simulated; actual WebGL loss uses WEBGL_lose_context.

Render checker: English/Vietnamese landing, download and getting-started, at
375×812, 768×1024 and 1440×900. Zero errors/warnings after contrast and touch
target repairs. Controller inspected the desktop opening and full mobile view.
Local screenshots/reports stay under ignored `.qa/website-overhaul-*`.

## Critique

Direction reads clear, practical, restrained. Weakest initial details were hero
wrapping, ledger contrast and the download migration touch target; all repaired.

| Item | Score / 2 | Rendered evidence |
|---|---:|---|
| Brief fidelity | 2 | Lit layered workbench is the opening signature; restrained green |
| Hierarchy | 2 | Two-line headline and one primary exploration action |
| Spacing | 2 | Related controls grouped; generous section separation |
| Alignment | 2 | Shared content edges; paired model and narrative columns |
| Typography | 1 | Clear sans-serif hierarchy; small model metadata on phones |
| Color discipline | 2 | Accent indicates selection, actions and evidence state |
| Depth and shape | 2 | Matte physical model; quiet surrounding hairline surfaces |
| States and feedback | 2 | Selection/loading/fallback, focus and disabled download |
| Responsiveness | 2 | Stacked mobile layout; no overflow; repaired touch target |
| Slop tells | 2 | No neon, RGB, fabricated metrics or decorative animation |
| Content | 2 | Concrete booking permissions, brief and evidence states |
| Craft | 1 | Functional Unicode model icons; no custom icon family |

Total: 22/24. Remaining visual trade-offs are stated above; no phone hardware FPS
or native accessibility measurement is claimed.

## Review repairs and limits

All three independent findings were repaired: pending-import page restoration,
retained control/view parity, Vietnamese visible/accessible model text. Review
report owns the independent assessment.

An initially exact PNG hash assertion failed after graphics loss. Direct pixel
inspection measured maximum channel delta 28 and mean 0.0374/255 at tile edges;
the visible view matched. Recovery now also verifies visible layer and compares
pixels with max 32 / mean below 0.1, without masking the image. Page-restoration
still requires exact image hash equality. Product CSP remains unchanged; the
test decodes its PNGs in a separate blank page because data URLs are prohibited
on the product pages.

## Publication

Pending hosted deployment and current QA build receipt. Public binary release
remains gated by native acceptance and exact approval, as recorded in the release
readiness report.

## Unresolved questions

- Clean Windows host and consenting pilot owners remain unavailable.
