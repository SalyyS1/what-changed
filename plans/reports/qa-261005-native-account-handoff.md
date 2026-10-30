# Native test-account handoff — 2026-10-05

Status: user chooses a separate Windows test account for remaining installed-app,
autostart/resource/accessibility and four-hour soak checks. No native gate closed.
This replaces the earlier unanswered choice, not permission to use current history.

## Authority and observed setup

Do not open, back up, mutate or reuse the existing current-user
%APPDATA%/dev.whatchanged.app directory. Do not create a local account, request
elevation, alter account groups or collect a password through chat. The app
itself must continue to run as a standard non-admin user.

Read-only local setup check finds Get-LocalUser and runas available. No local
account matches the scoped what-changed/wc-qa/codex-qa naming patterns; this is
not proof that no suitable account with another name exists. Root asks only for
the chosen test account name, or for the user to create/sign in once to a
standard account and report its name. Credentials stay outside the chat.

## Remaining handoff prerequisites

Confirm the account/profile and a safe interactive execution route before
installing/launching or writing its app history. Preserve the current user's
history and the existing rehearsal installer. Do not call the preserved
2026-09-26 rehearsal a current-source RC. Current-source release/license gates
and artifact identities remain independent prerequisites; do not bypass them.

After setup/artifact prerequisites, use the separate profile for the accepted
installed walkthrough/autostart, both working-set metrics, native accessibility,
resource and four-hour soak protocol. Record actual token/profile, artifact and
cleanup evidence without publishing a username or personal data. No test-account
setup or observed installed/soak behavior is claimed by this handoff record.

Software verification continues independently: page-capture focused13 and full
library525,36 callers, complementary lint and exactly one canonical10k actual
page smoke pass. [Page verification ledger](qa-261005-page-plan-capture-ledger.md).

Unresolved questions: chosen account name/profile, safe interactive route and
fresh verified artifact readiness; all native outcomes remain unobserved.
