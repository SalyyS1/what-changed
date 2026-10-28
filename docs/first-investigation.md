# First investigation

**Evidence state: available; the worked example uses deterministic synthetic data.**

The honest loop is:

1. On Home, describe the symptom in plain language, for example “the local
   service no longer answers on port 3000”.
2. Set the time window around the last known-good moment.
3. Review ranked suspects. Each card shows the evidence and confidence; the
   engine reports correlation and structural links, never a claimed cause.
4. Open an event for before/after details and related entities.
5. Use a checkpoint when the system is known-good, then compare it later.

Configuration values that are sensitive are represented as `[REDACTED]` or a
value-free change notice. The app does not retain a hash, digest, length or
encrypted comparison marker for a redacted value.

## Use a workspace baseline

In Settings, add the project root as a workspace and separately select any
folders whose file changes should be recorded. Choose **Mark working** while
the project works. In Investigations, select that workspace and describe the
symptom; its baseline supplies the last known-good time window.

New investigations include recorded file changes beneath the workspace's
currently registered root, plus recorded direct and related workspace events.
They can include file changes recorded before registration. Removing a
workspace changes future selections; saved investigation results remain fixed.
File rename membership follows the recorded destination path.

**Copy context for AI** asks for confirmation, then copies redacted Markdown
to the clipboard. Review it before pasting into an external service. The app
does not send it anywhere; the text remains in the clipboard until replaced.
