# Privacy

## The promise

**What Changed records that a sensitive value may have changed. It never records that value.**

`Sensitive configuration may have changed in .env` is the honest product when both observations redact to
the same literal. The old and new connection strings are not stored anywhere: not in the database, a log,
a diagnostics export, or a digest that could be attacked offline. Non-sensitive evidence such as ports,
versions and service states can be retained because it is what makes a change explainable.

Your history stays on this PC. There is no product account, sync, telemetry or crash reporting.
The recorder and offline workspace do not call a network service. An explicitly permitted
preview session runs a separate browser against the local preview you select.

## Owner workspace data (implementation in progress)

The approved workspace stores app descriptions, confirmed behavior rules, change
requests, preview configuration and candidate labels locally in a separate SQLite file.
Recognized secrets in text pass through the existing sanitizer before persistence
and idempotency hashing. Arbitrary user text is not guaranteed to be free of personal
information. Review the exact Markdown preview before copying; copied text leaves
application custody. Clipboard contents are never read automatically.

The development runner retains named test observations and uses temporary browser
profiles that may contain login/cache data. Screenshots require session opt-in,
an explicit capture and individual review before seven-day retention; they can be
listed and deleted. Portable memory exports exclude images and authentication.
Text redaction does not sanitize pixels or browser databases. Clean-Windows and
network-boundary acceptance remain open; remote targets are unavailable. See
[owner architecture](docs/ownvero/architecture.md).

Full deletion must clear both stores and their owned backups/quarantine/artifacts,
with a durable intent resumed before database startup if interrupted. Category/folder
deletions retain owner app memory. User-selected external exports and project source
are outside app-owned deletion. Unencrypted deletion cannot promise secure erase.

## What is stored

| Stored | Example |
|---|---|
| That a change happened, and when | A timestamp in UTC milliseconds |
| A human sentence describing it | `Docker Desktop updated 4.35.1 -> 4.36.0` |
| The entity it happened to | `service:MSSQLSERVER`, `port:tcp:0.0.0.0:5432` |
| Which key changed in a config file | `DATABASE_URL` |
| Version numbers, port numbers, service states, driver versions | `566.36 -> 566.45` |
| Paths of folders you chose to watch, and files inside them | `C:\Users\...\my-api\.env` |
| Relationships between entities | This process listens on that port; this config file belongs to that service |
| A collector's last-known state, so a restart can tell what changed while the app was closed | The set of installed services, as names and states — the same data the timeline already shows, not new data |

## What is not stored

| Not stored | Why |
|---|---|
| Config values classified as sensitive, old or new | The core promise |
| Environment variable values | The most common secret store on a developer machine |
| Process command lines | The richest source of accidental secrets; deliberately out of scope |
| Container environment variables | Where database passwords live; the endpoint that returns them is never called |
| File contents | Read only long enough to parse keys, then the buffer is zeroised |
| Hashes, digests, salted digests, or prefixes of redacted values | A digest is an offline-guessable value; see below |
| Anything about other users on the machine | Their profiles are blocklisted from watching |

One clarification about the config baseline, because "we store a shadow copy of your config file" would be
alarming if it were true. What is stored is the **key list plus a changed flag**, with every value already
through the redactor — so for a non-secret key like `server.port` the stored baseline holds `3000`, which is
the number the timeline shows you anyway, and for `DATABASE_URL` it holds `[REDACTED]`. The baseline exists
so a diff never has to re-read a previous version of the file from disk, which means the app never holds
plaintext history of anything.

## Why not even a hash

A short-value hash is guessable. Storing a digest of a password would let anyone with the database recover
the value by trying candidates, and storing a prefix leaks directly. Sensitive values are therefore
sanitized **before** comparison. When both observations become `[REDACTED]`, the recorder deliberately
refuses to claim an exact value change and emits a file-level uncertainty notice instead. Key addition or
removal remains structurally provable. There is no digest, salt, prefix or secret-derived comparison flag.

## How the promise is enforced

Not by discipline. By types.

The repository's batch-insert function accepts only a `Sanitized<T>` wrapper. That wrapper's field is private
and its only constructor lives in the sanitizer module. **Writing an unredacted event to the database is a
compile error.** There is no "remember to sanitize" step to forget during a late-night change to a collector.

Logs go through the same discipline one layer lower: the tracing writer itself scrubs every line, so a debug
statement that interpolates a raw value cannot produce an unscrubbed line on disk. The chokepoint is the
writer, not the caller.

## What gets redacted

Three independent rules, any of which is enough to replace a value with the literal string `[REDACTED]`.

**By key name.** The name is lowercased and its separators dropped, so `API_KEY`, `api-key`, `Api.Key` and
`apiKey` are one name. That normalised name is matched against a fixed list — password, passwd, pwd, secret,
token, apikey, authorization, authorisation (both spellings, because config files in the wild carry both),
cookie, session, sessionid, privatekey, clientsecret, accesskey, refreshtoken, bearer, credential,
passphrase. Then the name is split into words, on separators and on case transitions, and the **last** word
decides: key, keys, token, tokens, secret, secrets, password, passwords, passwd, pwd, cookie, cookies,
session, sessions, credential, credentials, passphrase, bearer, authorization, authorisation. A trailing
`id`, `ids`, `value` or `values` is a qualifier and is looked through, so `AWS_ACCESS_KEY_ID` and
`SESSION_ID` are still caught. A last word that is *glued* is caught too when it ends in password, passwd,
passphrase, token, secret or credential — `PGPASSWORD` and `NGROK_AUTHTOKEN` have no word boundary to find.

The last-word rule replaces the obvious suffix test on purpose. A plain "ends in key" test makes `MONKEY`,
`DONKEY` and `TURKEY` secrets; comparing whole words does not. `key` is deliberately excluded from the glued
list for the same reason.

**By value shape.** Recognisable credential formats are redacted regardless of the key they sit under,
because secrets get pasted into innocuously named fields: `sk-` vendor API keys, the token part of an
`Authorization: Bearer` value, JSON web tokens, AWS long-term and temporary access key identifiers, PEM
private key headers, `gh*_` source-host tokens, `xox*-` chat platform tokens, and credentials in a URI's
userinfo position. The URI rule keeps the scheme, host, port and database and replaces only the
`user:password` part, because an event that cannot say *which* database changed is not evidence of anything.

**By entropy.** A gated check on long opaque strings. All four gates must hold: at least 40 characters, not
all hexadecimal, not all digits, visibly encoded (mixed case or a `+`, `/` or `=`), and at least 3.6 bits of
Shannon entropy per character. The hex and digit exclusions are what let a sha256, a git commit, a container
id, a build number and a hardware serial through — those are exactly the values the product exists to show
you.

The rules are validated in both directions. Forty benign values — Windows paths with spaces, a UNC path,
semantic and four-part versions, KB article ids, a GUID, a registry CLSID, sha1/md5/sha256 digests, long
container ids, ports, PIDs, byte counts, timestamps, service and executable names, command lines, an IPv4
and an IPv6 address, a URL with no userinfo, and the four environment lines whose names look sensitive and
whose values are not (`API_PORT`, `KEYBOARD_LAYOUT`, `TOKENIZER_PATH`, `SESSION_TIMEOUT_MS`) — must
round-trip byte-identical, because a redactor that eats the useful data is as broken as one that leaks.

Deliberate over-redaction is accepted where the name cannot be told apart from a credential: `PUBLIC_KEY`,
`SSH_PUBLIC_KEY`, `PRIMARY_KEY`, `FOREIGN_KEY`, `SORT_KEY`, `PARTITION_KEY`, `CACHE_KEY`,
`IDEMPOTENCY_KEY`, `LICENSE_KEY` and `PRODUCT_KEY` all hide their values. Hiding a value you could have
seen costs you one line of a timeline entry; the opposite costs you a credential. The list is pinned by a
test, so it changes only on purpose.

## The leak test

A permanent build gate replays twelve shapes of realistic-but-synthetic secret through **both** sinks — the
structured path that ends at storage, and the log file — and searches each artifact for any eight-character
run of the original, because a partial leak is still a leak. It also replays each secret split across two
writes, since a formatter is free to emit one line in several calls. It may never be marked as ignored or
skipped. When a new secret shape is discovered in the wild, it becomes a permanent fixture rather than a
one-time fix; the suite asserts its own size so a fixture cannot be quietly dropped.

One of those assertions is made against the database file itself rather than against the code's output: all
twelve secrets are written through the real repositories — as events, as a detailed snapshot, as a
collector baseline and as a config baseline — the database is closed, and the **raw bytes of the file, its
write-ahead log and its shared-memory index are searched directly**. SQLite stores text verbatim, so a value
that reached a column would be found. The same test writes one ordinary value in the clear and requires to
find it, so the assertion cannot pass by reading the wrong file or an empty one.

## What redaction cannot do

Stated plainly, because a privacy claim with no stated limits is not credible:

- A low-entropy secret under an unusual key name can slip through. `db_conn = admin` matches no key rule, no
  value shape, and no entropy threshold. The mitigation is that the key-name rules and the shape patterns
  cover the overwhelming majority of real credentials, and every miss found becomes a permanent test fixture.
- An unquoted value is read up to the next space, so a password containing a space and no quotes would be
  partially replaced. Quoted values are read to the *matching* quote — an apostrophe does not close a
  double-quoted value, and a backslash-escaped quote is part of the value, because JSON writes one that way.
  A value whose quote is never closed is treated as running to the end of the line: the line ended inside the
  value, so there is no message after it to preserve. Where the two readings of a line disagree, the one that
  hides more wins, since this only ever applies to a key already judged sensitive. The alternative for the
  *unquoted* case — swallowing the rest of the line — would erase the message a log line exists for.
- **File paths are stored**, for folders you chose to watch and files inside them. They are what makes the
  timeline readable. They never leave the machine and they are excluded from the diagnostics export.
- **Entity names are stored** — service names, container names, application display names. They are the spine
  of the analysis graph.
- Structured diffing covers the common config formats. XML is the hardest of them, and if its diffing degrades
  the degradation is recorded in [ROADMAP.md](ROADMAP.md) rather than hidden.

## Two examples, verbatim from the product

The privacy screen shows these because concrete beats reassuring:

```
Sensitive configuration may have changed in .env
AWS_SECRET_ACCESS_KEY was added       value: [REDACTED]
```

## Where your data lives

`%APPDATA%\dev.whatchanged.app\` — the event database plus daily rolling log files with a short retention. The
path is resolved through the operating system's per-user application data API, never hardcoded. Nothing is
written anywhere else, and nothing is written outside your user profile.

## Retention

| Data | Default lifetime |
|---|---|
| Events | 30 days |
| Informational process start/stop events | 3 days (never longer than your Events setting) |
| Detailed snapshots | 7 days |
| Events classified important | 90 days |
| Snapshots belonging to a checkpoint | Kept until you delete the checkpoint |
| Log files | A small number of daily files |

Retention is adjustable in Settings. Independently of any lifetime, the database is capped at 1 GiB of live
data: past the cap the oldest informational events are removed first, down to 90% of the cap, and notable or
important events are never removed to meet it. Process start/stop events from short-lived helper programs
(shells, consoles, build tools) dominate a busy developer machine, which is why they expire sooner. Checkpoint snapshots are exempt from pruning on purpose: a marker you
named as "it worked here" has to still be comparable when you come back to it weeks later.

## Deleting your history

Three granularities, all in the privacy screen:

| Control | Effect |
|---|---|
| Stop watching a folder and delete its history | That folder's events, descendant config snapshots and baselines go |
| Delete one category's history | For example every network event, leaving the rest untouched |
| Delete everything now | Every history table, including checkpoints and investigations. Only theme and visualization preferences survive; onboarding is required again |

Deleting everything requires typing the word `delete`. It is irreversible and says so. Afterwards the product
says `Your history has been deleted. We're watching again from now.` The action
also requires a short-lived, one-use backend challenge. It stages a freshly
migrated database, removes app-owned migration backups, and clears the log
files after the replacement is verified. Scoped purges rebuild the search index
and reclaim pages where supported. The byte-scan test checks the database,
write-ahead log, shared-memory files and FTS shadow content for a fixture value;
normal unencrypted storage still cannot promise cryptographic erasure.

Uninstalling **keeps** your history folder by default. Deleting it is an opt-in checkbox in the uninstaller,
never the default. Silently deleting a user's history is not something this product does.

## Verify for yourself

The privacy screen is built to be checked, not believed:

- Live counts of what is stored, by category, with the database file size and the oldest record's date.
- The database path shown as selectable text, and a button that opens the folder in Explorer.
- The log file path and size.
- The plain statements: no account, no background network calls, no telemetry, no crash reporting.
  An explicitly initiated updater check is a separate future opt-in path and is unavailable until
  its signed HTTPS feed and public key are configured.

The recorder's acceptance walkthrough requires zero network bytes over a ten-minute session.
Preview-enabled candidates additionally require separate disabled-runner and authorized-egress
observations for the complete owned process tree. These observations are still open.

## The diagnostics export

Made for pasting into a bug report, so it is built from an explicit allowlist of fields rather than by
serialising internal structures. It contains application and OS versions, redacted settings, collector status,
counts by event type, recent log lines that already passed the scrubber, and recent errors.

It contains **no event content, no file paths, and no entity names**. A test greps the export for a fixture
entity name, a fixture path, and a fixture secret, and all three must be absent. The file header states the
same, so a user knows what they are sharing.

## Install-time network, once

The Windows installer downloads the WebView2 bootstrapper if that runtime is missing. That is an
**installation** step, disclosed in [README.md](README.md). Building from source also fetches dependencies
and packaging tooling. The Rust core has a dependency gate against HTTP clients, per
[SECURITY.md](SECURITY.md); that check does not certify a separately authorized browser's network
boundary. The opt-in updater scaffold is intentionally not wired to a feed yet.

## Notifications and the lock screen

Notification bodies use the same sanitized human summary the timeline shows. A toast therefore cannot surface a
secret on a locked screen, because the string it renders never contained one.

## Source of these rules

The complete rule set — every key name, every value pattern, the entropy threshold, and the test matrix — lives
in
[phase-03](plans/260829-0437-what-changed-desktop-mvp/phase-03-redaction-privacy-chokepoint.md).
The watched-folder constraints live in
[phase-10](plans/260829-0437-what-changed-desktop-mvp/phase-10-file-config-collector.md).
