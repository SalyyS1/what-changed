# Release-candidate screenshot set

**Evidence state: planned capture template; no personal or real machine data is committed.**

Run `scripts/capture-launch-assets.ps1 -Version 0.1.0 -Commit <rc-commit>` on
the tagged release candidate after enabling the deterministic synthetic fixture.
The script writes viewport, DPI, theme, version and commit metadata beside each
capture and refuses to overwrite an existing manifest for a different commit.

Required shots: calm Home, filtered Timeline, progressive event details, ranked
Investigation, checkpoint comparison, Change Map with 2D fallback, Privacy,
Diagnostics/degraded state, and tray. A release packet may attach PNG captures;
the repository keeps only this source-labelled template.
