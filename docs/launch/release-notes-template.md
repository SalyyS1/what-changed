# What Changed {{VERSION}}

**Evidence state: available template; release-specific evidence is planned per RC.**

## Highlights

- {{truthful_user_visible_change}}

## Evidence

- Source commit: `{{COMMIT}}`
- Installer SHA-256: `{{SHA256}}`
- Verification packet: [public go/no-go](../public-v1-go-no-go.md)

## Known limits

Use the versioned [known limitations](../known-limitations.md); do not add a
stronger claim in release notes.

## Install note

This unsigned build may show SmartScreen “Unknown publisher”. Runtime network
is not used; WebView2 bootstrapper download is install-time only when needed.
