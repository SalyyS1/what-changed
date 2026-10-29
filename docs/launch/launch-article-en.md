**Publication status: ARCHIVED — legacy What Changed material; not approved Ownvero copy.**

[Current owner-workspace messaging](messaging.md) owns the new draft positioning.
This historical copy predates the owner workspace and is retained for provenance;
it is not validated as current release, privacy, or brand guidance.

# It worked before. What changed?

**Evidence state: available canonical draft; publication is deferred until the P25 go/no-go approval.**

What Changed is a local Windows recorder for the moment after a familiar
sentence: “it worked yesterday.” It watches meaningful system signals, keeps a
bounded history, and turns a symptom into ranked evidence you can inspect.

The useful difference is persistence across sources. A configuration change,
a service transition and a listening-port change can sit next to one another in
the timeline. The investigation view explains why an item ranked highly; it
does not pretend correlation is proof of cause.

Privacy is part of the product shape. There is no account or runtime upload.
Sensitive values are redacted before persistence. If two values both become
`[REDACTED]`, the app does not keep a hash or other marker to tell them apart
later. It can still report structural additions and removals.

The preview is non-elevated and offline at runtime, with honest blind spots:
short-lived processes can be missed, some Windows channels require elevation,
and Docker live behavior needs a Docker Desktop machine for verification. The
installer is unsigned, so Windows may show “Unknown publisher”.

Read the [getting-started guide](../getting-started.md), then follow the
[first investigation](../first-investigation.md). The repository is the
canonical documentation source.
