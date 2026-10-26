# Security

## Scope

What Changed observes a Windows machine and stores a change history on that machine. It has no server, no
account, no remote surface, and no privileged component. That shape removes most of the categories a normal
threat model spends its time on, and concentrates the remaining risk in three places: what the application
reads, what it writes, and what the frontend can ask it to do.

## Assets worth protecting

| Asset | Risk if mishandled |
|---|---|
| The change history database | Reveals what software is installed, what ports are open, what projects exist |
| Config keys observed in watched folders | A leaked value is a leaked credential |
| The rolling log files | The classic accidental leak: the sanitizer covers events but not logs |
| The diagnostics export | Users paste it into public bug trackers |
| The user's machine itself | Anything the application can write, delete, or execute |

## Privilege boundary

The application runs under the invoking user's plain token. It never requests elevation, never installs a
service, never creates a scheduled task, and never writes outside the user's profile. Installation is
per-user and raises no UAC prompt.

That is a design constraint that was **measured, not assumed**. Of 35 capabilities probed under a plain
`BUILTIN\Users` token on the development machine, 31 worked and 4 did not. Only two of the four failed for
lack of privilege, and neither is a reason to elevate:

| Denied capability | Response |
|---|---|
| The process-start trace class returns access denied | Diff-poll processes once per second and disclose the sub-second gap in the UI |
| The `Security` event log channel is unauthorised | Do not subscribe it; carry a code comment so it is never re-added |

The other two failures are not privilege problems and are recorded separately in
[COLLECTORS.md](COLLECTORS.md): a WMI instance-creation subscription that registered
cleanly but delivered zero events in a three-second window (rejected on reliability),
and the Docker named pipe (absent because Docker is not installed on this machine).

There is no elevation path in the codebase to accidentally take.

## Read-only by construction

Not "read-only by policy" — the handles the application asks for cannot do anything else:

| Source | Access requested |
|---|---|
| Processes | Query-limited-information only, enough for the image name |
| Services | Enumerate-service on the manager, query-config on each service |
| Registry | Read-only key access, plus change notification |
| Docker engine | Five read endpoints. Nothing that can create, start, stop, exec, or remove |
| Filesystem | Read, only inside folders the user explicitly added, only to parse a small file |
| Event log | Subscribe on four named channels with id filters |

The one write outside its own data directory is the autostart entry: a single value under the current
user's own Run key, when the user turns the toggle on.

## Attack surface

| Surface | Threat | Mitigation |
|---|---|---|
| Watched folder paths | Path traversal, junction or symlink redirection into a system directory or another user's profile | Canonicalise; require a directory; reject reparse points and junctions; blocklist the Windows directory, both Program Files trees, the Microsoft ProgramData tree, other profiles, and the app's own data directory; cap root count and depth |
| The frontend | A compromised or malicious page script driving the backend | Closed 33-command allowlist, no dynamic dispatch, no eval, no SQL/shell/path passthrough; every command re-validates its input even for values Rust just sent |
| IPC flooding | A script hammering a command to fill the log or the database | The UI error-reporting command is rate-limited and its payload is scrubbed before it reaches the log |
| Config file content | A crafted `.env`, JSON, or XML file reaching the UI as markup | Values are redacted before storage, the UI renders everything as text nodes, and `{@html}` is banned codebase-wide by a test |
| Docker labels and container names | Attacker-controlled strings from a pulled image | Same path: sanitized, stored as text, rendered as text |
| A shadowing PATH entry | A hostile `node.exe` earlier in PATH gets executed by version probing | Only tools the user's own shell would already run; the resolved absolute path is stored in the event, so a shadowing entry becomes *visible* rather than silent |
| Process spawning | Arbitrary execution | Allowlist of 15 exact filenames, absolute resolved paths, no shell, one fixed argument, no window, short timeout, truncated output; timer-driven only — no IPC command can trigger a spawn |
| The diagnostics export | A user pastes their history into a public issue | Built from an explicit field allowlist of counts and versions; contains no event content, no paths, no entity names; asserted by a test that greps for fixture values |
| Log files | Secrets reaching disk through a debug line | Every log byte passes the same scrubber as events, at the writer, so an unscrubbed line is not reachable |

## Capability allowlist

Tauri capabilities are the real permission surface, so they are enumerated and gated rather than accumulated.
The full intended set is small:

| Capability | Granted | Why |
|---|---|---|
| Core default set | yes | Window and app basics |
| Window start-dragging | yes | The titlebar is application-drawn |
| Window minimize, toggle-maximize, close | yes | The three titlebar buttons are application-drawn too; `close` is intercepted in Rust and becomes hide-to-tray |
| Dialog open | yes | The watched-folder picker |
| Dialog save | yes | The explicit, user-initiated diagnostics export save dialog; `dialog:default` is not granted |
| Notification default | yes | Rust-owned notification policy and delivery adapter; the webview receives no generic notification API |
| Opener: reveal item in directory, scoped to the app data directory | not yet | Will make the privacy claim checkable by opening the data folder |

Two tests enforce this. The first fails the build if any permission identifier begins with `fs:`,
`shell:`, `http:`, `sql:`, `process:` or `updater:`. The second asserts the granted set is *exactly* the
rows marked granted above — a count alone would not notice one permission swapped for another, and a
permission that arrives with a copied snippet is precisely the case being defended against. A third
asserts the forbidden check can itself fail, because a gate that cannot fail proves nothing.

`updater:` remains on the forbidden list in this build because the signed HTTPS feed and public key do
not exist yet. A future release may add a narrowly scoped, user-initiated updater permission only after
the feed, key custody, signature verification and no-background-call tests are reviewed under D-09.

The notification plugin is registered in Rust and used from Rust — the one-time hide-to-tray balloon —
without granting the webview any notification permission. Registering a plugin and granting a capability
are separate acts, and only the second is reachable from a page.

## Redaction is a security control

The privacy chokepoint is described in [PRIVACY.md](PRIVACY.md), but its security property belongs here: the
repository's insert function accepts only a `Sanitized<T>` wrapper whose sole constructor lives in the
sanitizer module. Writing an unredacted event is a **compile error**, not a code-review finding. The same
applies to logs at the writer layer. A leak test covering a dozen secret shapes across both sinks is a
permanent build gate and may never be marked as ignored. One of its assertions is made against the bytes of
the database file rather than against the sanitizer's return value, so the property proven is "not on disk",
not "not in the output of the function under test".

As implemented, three things carry that property rather than one:

- **The wrapper.** `Sanitized<T>` in `src-tauri/src/privacy/sanitize.rs` has a private field and no `new`,
  no `From` and no `Deref`. Reading it is unrestricted; constructing one is possible only by calling the
  sanitizer, and taking the value back out is crate-private so that a sink cannot be handed a bare value.
- **The writer, not the caller.** `RedactingMakeWriter` sits under the tracing subscriber, so every byte the
  logging stack emits passes through `scrub_line` on its way to the rolling file and to the development
  console. A statement that interpolates a raw value cannot produce an unscrubbed line, and a value split
  across two `write` calls is buffered until the newline before it is judged — a test asserts the split case,
  because scrubbing half a secret is not scrubbing.
- **Order of the rules.** Shape rules run before the key/value scan, since some shapes are recognised by the
  words around them (`Bearer` before its token, a scheme before a URI's userinfo). Running the key rule first
  removed that context and left the token in the clear; the regression test for it is permanent.
- **No rule depends on what precedes a value.** A rule anchored on a word boundary before the pattern stops
  firing when a formatter writes the value straight after a letter — which the development console does, since
  its ANSI reset sequence ends in `m`. That let a `postgres://` credential reach a console while the same value
  was redacted in the log file beside it, and the leak suite now replays every fixture through both a plain and
  a coloured formatter so the asymmetry cannot come back.

Nothing derived from a secret is kept — no digest, no prefix, no length-preserving mask — so there is no
material an offline dictionary attack could work on. A config diff computes `changed` from the two values
before either is dropped, which is how the timeline can report that a credential changed without storing
either version.

The rule set is a table of literal patterns compiled once. A pattern that fails to compile is skipped rather
than panicking at the first event, and a unit test asserts the compiled count equals the table length: a
change recorder that crashes when it meets a secret would be worse than one running with eight rules, but a
rule silently lost to a typo would be worse than both, so the build fails instead.

Speed is a security property here too, because a scrubber expensive enough to be worth skipping is a scrubber
someone eventually skips. Two pre-filters answer "could anything match this line?" before the rules run — one
automaton over the patterns that contain a literal, and a byte scan for the long dense run the entropy rule
needs. Both are sound in one direction only: they may answer "not sure" and cost a full pass, and they may
never answer "definitely not" about a line a rule would have caught. Each is pinned by a test that feeds it
every term the real rules match on, because a fast path in front of a security control is a leak the moment it
is wrong in the other direction.

## Analysis engine

No network call, no model download, no inference service, no language runtime. The ranking is arithmetic over
rows already in the local database. There is no code path where a symptom description leaves the machine.

## Frontend hardening

- A Content Security Policy is set in the application configuration from the first commit.
- `withGlobalTauri` is off: there is no ambient bridge object on `window` for a script to find.
- Fonts and icons are local files. There is no CDN reference, so the CSP has nothing to allow.
- Development-only routes and the synthetic event generator are behind a build flag *and* a route-table
  filter, and a release audit test scans the built binary and the built assets to prove they are absent.
- Deep-link hash state is validated against the typed query schema before it is used.

## Dependency discipline

The dependency surface is treated as attack surface. Notable choices:

- No HTTP client is linked into the shipped Windows binary. A release gate resolves the dependency graph for
  `x86_64-pc-windows-msvc` and fails if `reqwest`, `hyper`, `ureq` or `isahc` appears in it. The gate resolves
  the *target* rather than grepping `Cargo.lock`, and that distinction is load-bearing: Tauri itself depends on
  `reqwest` for its Android, Linux and macOS paths, so those crates are present in the lockfile and absent from
  the Windows graph. A lockfile grep would fail on a clean scaffold forever and teach the next maintainer to
  ignore it. That check is the mechanical proof of the offline claim; "we do not make network calls" is
  otherwise just a sentence.
- Adding `bollard` for the Docker integration would break this for real, because its transport pulls `hyper`
  into the Windows graph. That is why it is rejected in favour of the hand-written transport below.
- The Docker transport is hand-written HTTP/1.1 over the named pipe rather than adding an HTTP client plus a
  pipe-transport crate for five read endpoints.
- Orbit camera behaviour is written directly against pointer events rather than importing the Three.js
  examples bundle, keeping that dependency to one audited package with no addons.
- The Windows Update history is read through the Update Agent's own COM interface on a dedicated thread with
  no pointer escaping it.

Exact versions are pinned in `package.json` and `src-tauri/Cargo.toml`, with the reasoning in
`docs/DECISIONS.md`.

## Build and release integrity

| Property | State |
|---|---|
| Code signing | **None.** There is no certificate. SmartScreen will say "Unknown publisher" |
| Publisher identity | Not spoofed, not invented, not claimed |
| Automatic updates | **None.** D-09 permits only a future explicit opt-in flow; this build has no configured feed/key |
| Install-time network | The WebView2 bootstrapper may download if the runtime is absent — the one install-time touch, disclosed in [README.md](README.md) |
| Release build | Development features and the synthetic seeder compiled out, asserted by a symbol and string scan of the shipped artifact |
| Source maps | Not shipped |

Until the signed feed and key custody exist, a user running an old build stays on it until they choose to
download a new one manually. This is the current safe state; enabling the opt-in path is a separate
release gate, not a hidden fallback.

## Destructive operations

There are exactly three, all user-initiated:

| Operation | Guard |
|---|---|
| Delete all history | Type `delete` and pass a short-lived one-use backend challenge; only reviewed theme and visualization preferences survive, and onboarding is required again |
| Delete one category's or one folder's history | Explicit action per scope |
| Uninstall | **Keeps** the data folder by default; deleting history is an opt-in checkbox, never the default |

No command can trigger deletion without that user action, and there is no scripting surface that could.

## Out of scope

Stated plainly, because a threat model that claims everything is a threat model nobody reads:

- **A compromised machine.** If an attacker already runs code as the user, they can read the database
  directly. The application is not a defence against local compromise and does not encrypt at rest.
- **Physical access.** The database is not encrypted. Full-disk encryption is the operating system's job.
- **Malicious input from the user's own configuration.** Watched folders are chosen by the user; the
  mitigations above are about blast radius, not about distrusting the user.

## Reporting a vulnerability

There is no published security contact or disclosure process yet — the project has no release. When there is
one, the channel will be documented here rather than assumed.
