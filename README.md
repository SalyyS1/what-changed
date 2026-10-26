# What Changed

**Evidence state: available public source preview; release publication remains gated.**

What Changed is a local-only Windows desktop app that records a plain-language history of changes around your PC and helps investigate “It worked before. What changed?” It correlates programs, ports, services, drivers, updates, network settings, startup entries, Docker state, and explicitly watched project folders without storing recognized secret values.

## What is ready

- Tauri 2 + Rust backend, Svelte 5 frontend, SQLite local history.
- Timeline, search, checkpoints, investigations, causal maps, diagnostics, privacy controls, watched folders, and project workspaces.
- AI Troubleshooting Pack: a redacted investigation context can be copied to the clipboard only after explicit confirmation.
- Opt-in updater scaffold: no background check, no configured production feed, and no release key in this repository.
- MIT project license and third-party notice inventory in [`THIRD_PARTY_NOTICES.md`](THIRD_PARTY_NOTICES.md).

## Status

This is a temporary public source preview. A current unsigned installer has been built and tested locally, but the release candidate is not published or signed. Installed acceptance, real upgrade/uninstall, full accessibility, runtime resource/network observation, four-hour soak, clean-tree release verification, and publication approval remain open. See the [release audit](docs/release-audit.md), [acceptance walkthrough](docs/acceptance-walkthrough.md), and [wiki index](docs/wiki-index.md).

## Quick start

```bash
bun install
bun run tauri dev
bun run verify
bun run tauri build
```

Windows prerequisites and the supported toolchain are documented in [Getting started](docs/getting-started.md) and [Contributing](CONTRIBUTING.md). The app requires Windows 10/11 x64, Rust MSVC, Bun, MSVC build tools, and WebView2.

## Privacy boundary

No account, cloud service, telemetry, or runtime network client is required. History stays under the OS-resolved `%APPDATA%\\dev.whatchanged.app\\` directory. Uninstall preserves history unless the user explicitly opts into deletion. Values recognized as secrets are redacted before persistence or comparison; paths and non-secret evidence may remain local so the app can explain a change.

## Documentation

The [temporary wiki index](docs/wiki-public-preview.md) links the product, workflow, architecture, privacy, security, troubleshooting, contribution, and release pages. The [ROADMAP](ROADMAP.md) records the accepted product direction and known deferrals.

## License

The application is MIT-licensed. Third-party components retain their own licenses; review [`THIRD_PARTY_NOTICES.md`](THIRD_PARTY_NOTICES.md) before redistributing a build.


