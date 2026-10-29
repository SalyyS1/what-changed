# Known limitations

**Evidence state: verified here where a named test exists; the remaining bullets are known limitations, not hidden failures.**

- The process collector can miss programs that start and exit between polls.
- Some Windows channels require elevation and are intentionally not read.
- Event-log records written while the app is closed are not reconstructed.
- Docker live behavior is not exercised on this development machine; the
  transport and scripted protocol path are tested instead.
- The preview installer is unsigned and may trigger SmartScreen.
- WebView2 installation can touch the network once when the runtime is absent.
- No automatic or background update check exists. The Settings opt-in updater scaffold remains
  unavailable until a signed HTTPS feed and release public key are configured.
- XML configuration may degrade to a file-level change when a safe structured
  parser is unavailable.
- Native notification click-to-event behavior still needs an installed-plugin
  check; source policy tests do not claim a Windows toast callback pass.
- The 1,000,000-row Phase 20 query run exceeded the current summary and search
  budgets on this machine. The 10,000 and 100,000 rows are recorded separately;
  no 1M performance pass is claimed.
- The four-hour soak, installed-app accessibility worksheet, installer
  upgrade/uninstall rehearsal and Resource Monitor network check remain open
  for a release candidate.

These are product boundaries, not hidden failures. See the owning root docs
and release audit for evidence and planned work.
