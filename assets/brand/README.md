# Brand assets

`icon-source.svg` is the canonical vector master for the Delta Bridge mark.
`delta-bridge.svg` is a compatibility alias kept for older asset references;
the exporter copies the canonical source rather than treating the alias as a
second design. Generated raster files are release artifacts and should be
produced by `bun run export:brand` (or `scripts/export-brand-assets.ps1`) in a
Windows build with ImageMagick. The source remains the authority; the exporter
writes content hashes and a rasterizer version so repeated exports are
deterministic and reviewable. It also renders the tray variants, NSIS header and
sidebar BMPs, and draft social/article rasters. Pass `-UpdateTauriIcons` only
when intentionally refreshing the tracked Windows/brand outputs and launch
social projection. The design review sheet is source-labelled, not publication
approval.
