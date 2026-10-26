# Release audit

**Evidence state: available checklist; publication remains NO-GO.**

Source-level checks cover formatting, type checking, Rust tests, frontend tests, bindings, no-network dependency review, bundle size, notice generation, and release artifact inspection. The current unsigned installer was built locally and is retained as a QA candidate; this repository does not publish it.

Open installed gates include no-UAC install observation, first-run onboarding, watched-folder redaction, investigation truthfulness, upgrade and uninstall behavior, accessibility, runtime network/resource observation, long soak, and clean-tree release execution.

Use [the acceptance walkthrough](acceptance-walkthrough.md) to record those checks against one named candidate. Do not infer installed behavior from unit tests or a successful build.
