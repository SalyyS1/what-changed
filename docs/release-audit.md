# Release audit

**Evidence state: available audit ledger; `Not-verifiable-here` rows are known limitations, never passes.**

This audit is evidence-first. `Done` means the named source test, artifact or
phase record proves the whole numbered requirement at its stated scope; it is
not a claim that an installed release candidate passed. `Not-verifiable-here`
means open: the named release/manual check is unrun, a required implementation
is missing, or a measured gate failed. The row says which. `Deferred-by-user`
is reserved for §26 and §75–§78. A `NO-GO` result is correct while any required
row is unresolved.

This ledger maps requirements to source and dated local evidence. A passing source test does not
close an installed-app, release-artifact or publication row. The release script,
the installer verifier and the manual walkthrough must identify the same candidate
before any row requiring installed or artifact evidence can move to `Done`.

The current local QA candidate is the rebuilt unsigned 0.1.0 installer after the
workspace investigation repair. Its private identity manifest is
`.qa/installer-artifacts-20261027-ui3.json`; installer and installed-file identity
checks passed. The [repaired-candidate report](../plans/reports/qa-261027-0619-repaired-installed-candidate.md)
records bounded native results and successful original-profile/startup restoration.
The [installed acceptance worksheet](acceptance-walkthrough.md) remains open.
Subsequent source accessibility fixes require a new installed candidate; these
observations do not establish complete acceptance, a clean-tree release or
publication approval. The 2026-09-26 rehearsal and the
[earlier 2026-10-27 candidate](../plans/reports/qa-261027-0059-installed-candidate.md)
remain evidence only for their respective artifacts.

## Non-negotiables

| Promise | Evidence | Status |
|---|---|---|
| No elevation | Per-user NSIS setting and installer walkthrough | Not-verifiable-here |
| No runtime network | `scripts/check-no-network.mjs` resolves the Windows target graph | Not-verifiable-here |
| No account/cloud/telemetry | command surface, capabilities and privacy docs | Done for source review |
| One process | Tauri app model and architecture tests | Done for source review |
| Redaction before persistence | privacy tests and sanitizer boundary | Done for source review |
| No causal claim | analysis tests and forbidden-word checks | Done for source review |

## Requirement traceability

The subjects below follow the [numbered requirements digest](../plans/260829-0437-what-changed-desktop-mvp/reports/ultra-evidence-packet.md), not a citation count. `Pnn` identifies the owning phase in the [accepted plan](../plans/260829-0437-what-changed-desktop-mvp/plan.md). A source test proves only its stated behavior; an installed-app clause stays open until the [15-step walkthrough](acceptance-walkthrough.md) names the same candidate.

| Requirement and subject | Owning phase | Named test or manual check | Status |
|---|---|---|---|
| §1 PC flight recorder and plain-language answer | P06, P07, P15 | `since_last_restart.rs`; walkthrough #4–6 and #12 still unrun | Not-verifiable-here |
| §2 Simple and advanced audiences in one app | P07 | `details-drawer.test.ts`; walkthrough #4 and #6 still unrun | Not-verifiable-here |
| §3 Tauri, Svelte, Rust and local stack | P01, P16 | `check-versions.mjs`, `package.json`, `Cargo.toml`, production build recorded in P01 | Done |
| §4 Customized UI primitives and secret-safe local logging | P01–P03 | `redaction_leak.rs` log scan; P01 logging and P02 design-system gates | Done |
| §5 One binary; future sidecar protocol only | P01, P08 | `architecture.rs` process boundary; `COLLECTORS.md` external protocol review | Done |
| §6 Windows 10/11 x64 with core/platform split | P01, P08 | `architecture.rs` platform-boundary test; Windows target build | Done |
| §7 Tray lifecycle, menu and optional autostart | P05, P19 | `wiring.rs`; walkthrough #9 and installed autostart-minimized check still open | Not-verifiable-here |
| §8 Typed UI → service → repository architecture | P04, P05, P08 | `architecture.rs` and `commands_contract.rs` | Done |
| §9 Collector-independent normalized event model | P04 | `normalize.rs` and `migrations.rs` | Done |
| §10 SQLite tables, indexes and no full file contents | P04 | `migrations.rs`, `architecture.rs` schema/table coverage | Done |
| §11 Retention defaults, maintenance and storage estimate | P04, P19 | retention tests in `repository.rs`; Settings storage estimate walkthrough still open | Not-verifiable-here |
| §12 Collector lifecycle, health and isolation contract | P08 | `collector_runtime.rs` and `collector_isolation.rs` | Done |
| §13 Explicit file watches and redacted config diffs | P03, P10 | `collector_files.rs`, `config_diff.rs`; walkthrough #3–4 still unrun | Not-verifiable-here |
| §14 Process lifecycle and privacy-safe metadata | P09 | `collector_process.rs`; P09 real Notepad/Python smoke record | Done |
| §15 Listener changes and process–port edge | P09 | `collector_listeners.rs`, `relationships_process_port.rs`; P09 real port smoke | Done |
| §16 Windows service state and startup changes | P11 | `collector_services.rs`; P11 real-OS steady sweep record | Done |
| §17 Installed application/version changes | P11 | `collector_applications.rs`; P11 real registry sweep record | Done |
| §18 Windows Update install/failure/reboot evidence | P12 | `collector_updates.rs`; P12 real WUA sweep record | Done |
| §19 Driver versions and friendly device names | P12 | `collector_drivers.rs`, `driver_naming.rs`; P12 SetupAPI sweep | Done |
| §20 DNS, route, proxy, gateway, VPN and IP changes | P13 | `collector_network.rs`; WinHTTP machine-proxy coverage remains a documented gap | Not-verifiable-here |
| §21 Startup entries and feasible startup tasks | P13 | `collector_startup.rs`; P13 provider gate | Done |
| §22 Relevant Windows Event Log records only | P13 | `collector_eventlog.rs` and `eventlog_storm.rs`; P13 provider gate | Done |
| §23 Optional local Docker collector | P18 | `collector_docker.rs`, `docker_protocol.rs`; live daemon check unavailable here | Not-verifiable-here |
| §24 Opt-in Git/development project changes | P18 | `collector_dev_env.rs`, `project_repository.rs` | Done |
| §25 Labeled checkpoint snapshot and comparison | P14 | `checkpoint_create.rs`, `checkpoint_compare.rs`; P14 real Windows capture | Done |
| §26 Configurable health checks | P19, roadmap | User-selected later-MVP decision recorded in `ROADMAP.md`; no MVP test | Deferred-by-user |
| §27 Deterministic investigation pipeline | P15 | `analysis_engine.rs`, `investigation_service.rs` | Done |
| §28 Transparent relevance score and rank reasons | P15 | `analysis_scoring.rs`, `analysis_ranking_fixtures.rs` | Done |
| §29 Evidence-classed chains without causal overclaim | P15 | `analysis_chains.rs`, `no_causal_overclaim.rs` | Done |
| §30 Persisted cross-source entity relationships | P04, P09–P18 | `relationships_process_port.rs`, `config_relationships.rs`, `system_map_payload.rs`; Docker-live edge remains open | Not-verifiable-here |
| §31 Prominent symptom input and deterministic parsing | P07, P15 | `home-screen.test.ts`, `symptom_lexicon.rs` | Done |
| §32 Optional AI provider abstraction with deterministic default | P15 | `analysis::explain::{ExplanationProvider, DeterministicExplanationProvider}` and provider parity/injection tests; deterministic provider performs no I/O | Done |
| §33 Local privacy, no spyware/account/cloud/telemetry | P03, P19, P21 | `privacy_purge.rs`; installed privacy walkthrough #10–12 and runtime-network #15 open | Not-verifiable-here |
| §34 Central redaction before persistence and logging | P03 | `redaction_leak.rs`, `redaction_leak_config.rs` | Done |
| §35 Narrow Tauri permissions and Rust validation | P05, P21 | `capability_allowlist.rs`, `commands_contract.rs`, `path_validation.rs` | Done |
| §36 Non-admin process, no silent elevation | P08, P21 | `check-nsis-script.mjs`; fresh-install walkthrough #1 still unrun | Not-verifiable-here |
| §37 CPU, RAM and bounded disk budget | P20 | `perf_scale.rs`, `measure-memory.ps1`; current-source 1M resource and full B2 record open under D-15 | Not-verifiable-here |
| §38 60 FPS, paused/disposed 3D and WebGL fallback | P16, P20 | `change-map-frame.test.ts`, `change-map-2d.test.ts`; installed frame gate open | Not-verifiable-here |
| §39 DOM-first Three.js, Home ambient and causal graph | P16 | `change-map-renderer.test.ts`; source check finds no Home ambient scene or transition | Not-verifiable-here |
| §40 Calm premium visual direction | P02, P22 | P02 screen review; P22 Windows tray/theme/installer design inspection open | Not-verifiable-here |
| §41 Design tokens and dark/light/system themes | P02 | `contrast.test.ts`, `theme-bootstrap.test.ts` | Done |
| §42 Purposeful motion and reduced-motion support | P02, P16, P20 | `reduced-motion.test.ts`; installed reduced-motion review open | Not-verifiable-here |
| §43 Home status, prompt, counts and important changes | P07, P16 | `home-screen.test.ts`; walkthrough #2 still unrun | Not-verifiable-here |
| §44 Filtered, clustered, virtualized Timeline | P06, P07 | `timeline-screen.test.ts`, `timeline-list.test.ts`; walkthrough #4 open | Not-verifiable-here |
| §45 Ranked Investigation with evidence sequence | P15 | `investigation-screen.test.ts`; walkthrough #5–6 open | Not-verifiable-here |
| §46 Event details and technical identifiers | P07 | `details-drawer.test.ts`; walkthrough #4 open | Not-verifiable-here |
| §47 Checkpoint cards, rename/delete/compare | P14 | `CheckpointsScreen.test.ts`; walkthrough #7 open | Not-verifiable-here |
| §48 Advanced System Map, optional to simple flow | P17, P20 | `system_map_payload.rs`, `SystemMapScreen.test.ts`; walkthrough #8 open | Not-verifiable-here |
| §49 Settings sections and explained collector toggles | P19 | `SettingsScreen.test.ts`, `CollectorList.test.ts`; installed Settings review open | Not-verifiable-here |
| §50 Short privacy/monitoring/autostart onboarding | P19 | `onboarding_state.rs`; fresh-install walkthrough #2 open | Not-verifiable-here |
| §51 One app with progressive technical disclosure | P07, P20 | `details-drawer.test.ts`; keyboard/Narrator walkthrough #13–14 open | Not-verifiable-here |
| §52 Indexed global search across entities/events/labels | P06, P07 | `query_service.rs`, `search-palette.test.ts` | Done |
| §53 Conservative notifications, no routine spam | P19 | `notification_policy.rs`; native toast-click check still open | Not-verifiable-here |
| §54 Calm importance levels without false panic | P06, P19 | `importance_calm.rs`; installed Home/Timeline review open | Not-verifiable-here |
| §55 Modular project and documentation boundaries | P01 | `architecture.rs`; P01 project-tree review | Done |
| §56 Typed narrow frontend ↔ Rust commands | P05 | `bun run check:bindings`, `commands_contract.rs` | Done |
| §57 Named live events and throttled UI updates | P05, P07 | `stores.test.ts`, `timeline.test.ts`; installed live-event walkthrough #4 open | Not-verifiable-here |
| §58 Typed errors and collector failure isolation | P08, P18, P19 | `collector_isolation.rs`, `collector_docker.rs` absent-pipe case | Done |
| §59 Rust/frontend tests and port/config ranking fixture | P15, P25 | `analysis_ranking_fixtures.rs`; full current-candidate verification and practical E2E open | Not-verifiable-here |
| §60 Secret fixture redaction tests | P03 | `redaction_leak.rs`, `redaction_leak_config.rs` | Done |
| §61 10k/100k/1M and burst performance suite | P20 | [Frozen guarded-recurrence query matrix](../plans/reports/qa-261005-recurrence-rolling-query-matrix.md) passes all 24 cells; canonical size/retention evidence is routed by [performance evidence](performance-baseline.md); seed, SQL-plan branch parity, payload sensitivity, resource/soak and installed gates remain open | Not-verifiable-here |
| §62 Keyboard, screen reader, contrast and DOM fallback | P20 | `contrast.test.ts`, `focus-order.test.ts`; installed Narrator worksheet open | Not-verifiable-here |
| §63 Fully offline core and bundled assets | P21 | `check-no-network.mjs`; Resource Monitor walkthrough #15 open | Not-verifiable-here |
| §64 Manual update control and migration-safe update path | P21 | `SettingsScreen` explicit opt-in control, typed `checkForUpdates(optIn)`, `update_command.rs`; signed feed/key and installed upgrade check remain open | Not-verifiable-here |
| §65 Per-user installer, shortcuts and safe upgrade/uninstall | P21 | `verify-installer.ps1`, `check-nsis-script.mjs`; rebuilt ui3 installer and installed-file identities verified; complete walkthrough #1 and real upgrade/uninstall open | Not-verifiable-here |
| §66 Separate production signing and no checked-in secret | P21 | `deployment-guide.md` signing section; `SECURITY.md` source review | Done |
| §67 Bun development commands and standalone app | P01, P21 | `package.json` scripts and rebuilt standalone ui3 candidate; clean-tree release and complete installed acceptance remain open | Not-verifiable-here |
| §68 Eight current root authority documents | P01, P23 | `check-docs.mjs`; release-candidate content reconciliation still open | Not-verifiable-here |
| §69 Prescribed phase ordering through release | P01–P25 | `plan.md` dependency graph; P19–P25 acceptance still open | Not-verifiable-here |
| §70 Six states on every major screen | P02, P20 | `state-shell.test.ts`; installed 900×600 six-state pass open | Not-verifiable-here |
| §71 Desktop layout down to 900×600 | P01, P07, P20 | Tauri min-size config; walkthrough #13 open | Not-verifiable-here |
| §72 Exact calm Home microcopy | P07, P19 | `home-screen.test.ts`; exact-copy inspection on installed RC open | Not-verifiable-here |
| §73 Driver/crash investigation tone and caution | P15, P24 | `analysis_ranking_fixtures.rs` driver scenario; walkthrough #5–6 open | Not-verifiable-here |
| §74 Config → restart → port-change 502 example | P10, P15, P18 | `config_relationships.rs`, `analysis_ranking_fixtures.rs`; Docker-live demonstration open | Not-verifiable-here |
| §75 Future-feature bundle; no MVP build | roadmap, P21 | `ROADMAP.md` future-scope review; bundled digest heading | Deferred-by-user |
| §76 Future-feature bundle; no MVP build | roadmap, P21 | `ROADMAP.md` future-scope review; bundled digest heading | Deferred-by-user |
| §77 Future-feature bundle; no MVP build | P17, roadmap | `system_map_payload.rs` relationship readiness; bundled digest heading | Deferred-by-user |
| §78 Future-feature bundle; no MVP build | roadmap, P21 | `ROADMAP.md` future-scope review; bundled digest heading | Deferred-by-user |
| §79 Rust/TypeScript/CSS coding standards | P01–P25 | `bun run verify`; clean final source gate still open | Not-verifiable-here |
| §80 Deliberate dependency policy | P01 | `docs/DECISIONS.md` accepted/rejected dependency review; `bundle-size.mjs` | Done |
| §81 Rust DTO source of truth and TS parity | P04, P05 | `bun run check:bindings` | Done |
| §82 Documented threat model and mitigation | P01, P03, P05 | `SECURITY.md`, `capability_allowlist.rs`; current-candidate threat review open | Not-verifiable-here |
| §83 WAL, migration backup and corruption recovery | P04, P19, P21 | `migrations.rs` v13→v14 small repair fixture passes; `db_recovery.rs`; 1M upgrade timing/free-space and installed two-version migration open | Not-verifiable-here |
| §84 Diagnostics status and redacted export | P19 | `diagnostics_export.rs`, `DiagnosticsScreen.test.ts`; walkthrough #11 open | Not-verifiable-here |
| §85 Hidden dev simulation, absent from normal release | P18, P21 | `DevSimulationPanel.test.ts` and prior artifact scan; current-source normal-release scan open | Not-verifiable-here |
| §86 Inspect/plan/implement without discarding work | P01–P25 | `plan.md` implementation log; final source/plan review open | Not-verifiable-here |
| §87 No fake collector or falsely closed phase | P01–P25 | Phase acceptance checklist review; live Docker and manual gates explicitly open | Not-verifiable-here |
| §88 Verify and report after every phase | P01–P25 | Phase verification sections; P19–P25 final reports still open | Not-verifiable-here |
| §89 Commit-sized work | P01–P25 | `git log --oneline` phase-boundary review still open for current tail | Not-verifiable-here |
| §90 Docs-first foundation, polished shell and synthetic feed in first task | P01, P02, P05, P06 | P01/P02 shell records; `simulate_event` landed in P06, after the first task's specified sequence | Not-verifiable-here |
| §91 Immediate “Oh. THAT changed.” product loop | P07, P15, P25 | Walkthrough #3–8 and first-launch timed task still unrun | Not-verifiable-here |

The digest combines §3–§4 and §75–§78 under shared headings. Those numbers have separate rows above; the latter four inherit the single user-deferred future-feature decision. `ROADMAP.md` calls §76 Fix Memory, §77 Impact and §78 Can I Delete This?, while P21 calls §76 Impact and §77 blast radius. Their individual feature labels need reconciliation in the plan before a narrower traceability claim is possible.

Blocking source findings: §39 has no Home ambient scene or focused transition (an accepted G7 omission); §64 has an explicit opt-in scaffold, but no signed HTTPS feed/public key or installed upgrade check yet. These are open release inputs and manual gates, not hidden claims. §90's synthetic feed was implemented in P06, after the prescribed first task, so the historical sequencing claim cannot be marked `Done`. §65 also remains open: the generated NSIS script proves shortcut sections and an opt-in data-deletion guard, but a real upgrade/uninstall and the optional-desktop choice have not been demonstrated on an installed candidate.

## Measured evidence

Phase 09, 15, 16 and 17 report their own numbers in
[docs/PERFORMANCE.md](PERFORMANCE.md). Phase 20 owns the 10k/100k/1M corpus,
query-plan, soak and accessibility measurements. Existing entries in
[docs/PERFORMANCE.md](PERFORMANCE.md) are historical or phase-scoped. The
2026-09-26 query/size run predates migration 014 and is not current-release proof.
The [2026-10-05 frozen-source query matrix](../plans/reports/qa-261005-recurrence-rolling-query-matrix.md)
passes all three scales; it is local source evidence, not an installed release
or a pass for seed, SQL-plan branch parity and resource/manual requirements.
The B2 decision requires both total
and private working set at each checkpoint; no memory pass is claimed until those
readings exist.

## Local source checks

These checks protect documentation and source boundaries but do not replace the
release gates above:

- `node scripts/check-docs.mjs` validates the Markdown route and relative links.
- `node scripts/check-brand-assets.mjs` validates the canonical brand source set.
- `node scripts/check-launch-copy.mjs` validates bilingual launch copy evidence labels and anti-claims.
- `node scripts/check-versions.mjs` validates package and Cargo version agreement.
- `scripts/check-no-network.mjs` is authoritative only when fed the target-resolved
  Cargo graph by `scripts/release.ps1`; a lockfile search is not equivalent.

## Known gaps at this audit

- Docker live daemon behavior is not verified on this machine.
- The installed-app walkthrough has partial native evidence and an ongoing
  repaired-candidate session; the complete worksheet and Resource Monitor
  acceptance remain open.
- The unsigned installer and WebView2 bootstrapper behavior need a Windows GUI
  record.
- P19 purge/export/onboarding and P20 resource/soak/accessibility gates need named
  current-candidate evidence before publication. The P20 query and database-size/reclaim
  gates have dated 10k/100k/1M local measurements, including the frozen-source
  2026-10-05 query pass, not a complete installed release pass.
- P21 has a rebuilt unsigned installer and verified installed-file identity.
  Complete install/shortcut, upgrade and uninstall acceptance still require a
  finished candidate record; older rehearsal artifacts are not publication approval.
- The [third-party source review](../THIRD_PARTY_NOTICES.md) is approved and the
  notice payload generates/checks successfully. Frozen installer and installed
  notice binding remain separate acceptance work.
