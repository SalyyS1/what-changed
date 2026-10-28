# Offline owner workflow

**Evidence state: available in development source; verification and installed acceptance remain in progress.**

This documents development source. The existing unsigned installer does not include
or validate this workflow. [Implementation status](../../plans/261027-0817-ownvero-owner-workspace-web-wiki/plan.md)
owns remaining verification and release gates.

## Keep the behavior you want

Open **My apps**, create an app and describe its purpose. Add capabilities such as
bookings, customer accounts or staff scheduling. Confirm each behavior rule in owner
language. A rule edit creates a revision; old wording remains available.

Linking a local troubleshooting project is optional and does not enable collectors.
The owner workspace is available separately from legacy monitoring onboarding.
The synthetic booking example is read only and contacts no preview.

## Request a change and hand it to your coding AI

Create a change request with intent, an example, scope and the rules to preserve.
Save it before leaving; reopening uses its saved ID and version. Version conflicts
require reloading rather than overwriting another edit.

Review the generated Markdown and its omissions before copying. It carries the exact
request version and confirmed rule revisions. Target details are excluded unless
selected. The brief has a 32 KiB limit; an oversized brief returns an actionable error
instead of silently cutting rules. The app never reads the clipboard.

## Check a named candidate manually

Configure a test/local environment and select a candidate. Configuration does not
contact a URL. Bind the saved request to that candidate explicitly.
For a separate test API, add its exact origin to the same environment. A maximum
of 16 distinct origins is supported; editing a label preserves the saved origins.
Scenario route choices use the selected candidate's current environment.

Author a scenario using the bounded recipe controls. Review old and proposed criteria,
including exact locators, expected text, response fields and roles, before confirming
a revision. Freeze a manual run, perform its steps yourself and record observer,
time, step observations and each assertion outcome.

Missing output is not a pass. Observations are labelled **owner manual** and retain
failed, blocked and not-run results. Retrying creates another attempt.

Changing a request, rule, scenario, candidate or environment makes the relevant old
evidence stale. Manual observations do not establish deployed build identity; a URL,
commit or owner-entered label alone does not change that. Review can record rejection,
deferral or acceptance with explicit exceptions, a reason and acknowledgment of each
unresolved scenario and unknown build. It does not turn failed checks into passes.

## Project memory and portable files

Save a project decision with its rationale, author and source. Editing it supersedes
the previous revision. Filter history by capability, change or provenance.

Preview a project memory bundle before saving. It contains versioned JSON, readable
Markdown, recipe definitions and a SHA-256 file inventory. It does not include auth,
browser sessions, source roots, raw logs or capture files. Default export substitutes
target URLs, local paths and owner names. Review free text: automatic redaction cannot
recognize every possible secret.
Saving checks that the workspace still matches the preview. If it changed, prepare
and review another preview. An existing file is replaced only after confirmation.

Import uses a native file picker and validates the complete bundle before writing.
Choose a name for a separate app and explicitly confirm each imported rule revision.
Original scenarios, targets, decisions and observations remain readable historical
data. Configure a new target and confirm scenarios before checking again. Import
does not run recipes or restore permission or a current verification result.

The supported ZIP profile has four stored regular files, no compression or extra
fields, and a 16 MiB maximum. Names, CRCs, SHA-256 inventory and schema are checked
in memory; archive entries never become filesystem paths. Repacked compressed ZIPs
are rejected with a supported-format error. Checksums detect corruption, not truth.

## Local data and deletion

Owner records use `owner-workspace.sqlite` beside the legacy database. One unavailable
store is reported separately and is preserved. Full privacy deletion closes mutation
admission, drains workers and queued logs, clears owned data and pending previews,
then resets onboarding. External source projects and files you exported are retained.
Deletion errors remain visible and retriable. Full deletion while legacy recovery is
required currently returns an error rather than claiming zero unavailable counts.
Deletion also includes app-owned backup/restore generations; files you saved outside
the app data directory remain yours. The internal restore API has no user-facing
screen yet and does not establish installed upgrade/downgrade acceptance.

## Optional preview checks and visual evidence

Development source adds a browser-check panel beneath scenario selection. Bind a
candidate, select confirmed scenarios, enter synthetic fixture values and review
every allowed request. A separate prompt requires test-environment authorization,
isolated data and acknowledgment of temporary login storage. Nothing runs while
editing configuration or reading a brief. Leaving the workspace requests revocation.
If native termination is not confirmed, a persistent app-wide banner retains the
exact session and retries cancellation; manual retry remains available across screens.

Local previews currently require an explicit `127.0.0.1` or IPv6 loopback origin.
Remote previews remain unavailable. A matching packaged runtime is required; there
is no automatic browser download or use of a personal profile. Sign in visibly for
each test role, then continue. Results retain exact candidate/scenario versions;
missing build markers remain unknown. Review and accept/reject/defer separately.

Screenshot capture is off by default. Enable it for a session, request an image,
inspect it, then retain or discard it. Retained images expire within seven days
and are absent from portable memory and runnable test ZIPs. Review the generated
test files before saving a runnable ZIP; it requires its documented independent
setup and does not carry app permissions.

Use the journey and capability map to follow rules through scenarios to evidence.
The 2D, optional 3D and keyboard list show the same facts and inspector. Before/after
comparison names incompatible revisions instead of presenting an improvement score.
Clean-Windows packaging, network confinement, native performance and the real-owner
rehearsal remain open acceptance checks after feature implementation.
