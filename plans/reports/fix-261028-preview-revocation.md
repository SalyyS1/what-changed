# Preview revocation recovery

Status: DONE

## Cause and acceptance

`PreviewRunnerStore.dispose()` discarded the only frontend session handle after a fire-and-forget cancellation whose error was swallowed. The stale-grant branch also lost its exact session when cancellation failed after a generation change. Navigation therefore could leave an active native preview without usable cancellation controls.

Required behavior: workspace teardown stays synchronous; a failed or unconfirmed revocation retains its app/run identity and retry controls across navigation until native status confirms that exact session is terminal. Root teardown stops frontend retry timers.

## Changes

- Added `src/lib/stores/preview-revocations.svelte.ts`: session-specific cancellation/status recovery, exact app/run checks, one in-flight attempt per handle, retry delays capped at ten seconds, and root attachment cleanup. Native cancellation acknowledgment alone never removes a handle.
- Updated `src/lib/stores/preview-runner.svelte.ts`: navigation, manual cancellation and stale grants hand existing sessions to recovery. Generation guards remain; injected native dependencies get isolated recovery state unless a shared owner is explicitly supplied.
- Added `src/features/verification-runner/PreviewRevocations.svelte` and narrow integration in `src/app/App.svelte`: named persistent region, status/error announcements, exact session IDs and manual retry controls across routes.
- Added regression coverage for cancellation failure after navigation, late grant failure after a newer preparation, terminal confirmation, mismatched status, same run ID in distinct apps, adapter preservation, overlapping requests, capped retries, detach/remount and actual App navigation/unmount.

## Verification

`bun run test src/lib/stores/preview-runner.test.ts src/lib/stores/preview-revocations.test.ts src/app/App.test.ts --pool=threads --maxWorkers=1`

```text
Test Files  3 passed (3)
     Tests  29 passed (29)
Start at  12:34:54
Duration  41.53s
```

`bun run check` — exit 0; `svelte-check found 0 errors and 0 warnings`; TypeScript check passed.

Initial regression assertions used plain JavaScript errors; existing IPC normalization correctly sanitized them. Test failures were repaired using native error DTO shapes. An initial type check also caught tuple inference in the new timer test; the test now uses a readonly tuple list.

## Review and boundaries

Local review: recovery only calls cancellation/status for an already obtained session. It does not prepare or grant execution, expand preview origins, or alter native authorization. UI teardown does not await IPC. App unmount clears timers, while native lifecycle ownership remains responsible for process cleanup on app exit. No native packaging or installed-app execution was performed by this subtask.

## Unresolved questions

None.
