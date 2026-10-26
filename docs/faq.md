# FAQ

**Evidence state: available product answers; release-specific behavior remains a known limitation until the named RC is published.**

## Does it upload my history?

No. Runtime collection is local-only. The installer may download WebView2 if
Windows does not already have it.

## Does it store my `.env` values?

No. Recognized sensitive values are redacted before comparison and persistence.
The UI can say that a sensitive key may have changed without naming its value.

## Does an event prove the cause?

No. The investigation engine ranks evidence and structural relationships. It
does not claim that one event caused another.

## Why is Docker unavailable?

The collector reports the exact unavailable state when Docker Desktop is not
running or installed. It resumes probing without filling the log with errors.

## How do I remove my data?

Use the privacy controls in the app. Read the scope and limitations before
confirming; keep a copy only when you intentionally want one.
