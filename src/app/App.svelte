<script lang="ts">
  import { onMount, untrack } from 'svelte';
  import { theme } from '../lib/design/theme.svelte';
  import { collectors } from '../lib/stores/collectors.svelte';
  import { monitoring } from '../lib/stores/monitoring.svelte';
  import { router, type RouteParams } from '../lib/stores/router.svelte';
  import { settings } from '../lib/stores/settings.svelte';
  import { backendAvailable, setTrayForcedColors } from '../lib/tauri/client';
  import { onTrayNavigate, type Unlisten } from '../lib/tauri/events';
  import { watchForcedColors } from '../lib/tauri/tray-appearance';
  import AppShell from './AppShell.svelte';
  import RecoverySurface from '../features/home/RecoverySurface.svelte';
  import OnboardingFlow from '../features/onboarding/OnboardingFlow.svelte';
  import { routeById, type RouteDef, type RouteId } from './routes';
  import { navScreens } from './screens';
  import MyAppsScreen from '../features/owner-workspace/MyAppsScreen.svelte';
  import AppOverviewScreen from '../features/owner-workspace/AppOverviewScreen.svelte';
  import ChangeRequestScreen from '../features/change-request/ChangeRequestScreen.svelte';
  import { changeRequest } from '../lib/stores/change-request.svelte';
  import Button from '../components/ui/Button.svelte';
  import Dialog from '../components/ui/Dialog.svelte';
  import { projects } from '../lib/stores/projects.svelte';
  import { previewRevocations } from '../lib/stores/preview-revocations.svelte';
  import PreviewRevocations from '../features/verification-runner/PreviewRevocations.svelte';

  let pendingNavigation = $state<{ route: RouteDef; params: RouteParams } | null>(null);
  let ownerChangeDirty = $state(false);
  const ownerRoute = $derived(['owner-apps', 'owner-app', 'owner-change'].includes(router.current.id));
  $effect(() => { if (router.current.id === 'owner-app' && backendAvailable()) untrack(() => void projects.load()); });

  function setupNavigationGuard(): () => void {
    return router.setNavigationGuard((route, params) => {
      if (!(ownerChangeDirty || changeRequest.dirty) || router.current.id !== 'owner-change') return true;
      return route.id === 'owner-change' && params.appId === router.params.appId && params.changeId === router.params.changeId;
    }, (route, params) => { pendingNavigation = { route, params }; });
  }

  function continueNavigation(): void {
    const pending = pendingNavigation;
    pendingNavigation = null;
    if (pending) { ownerChangeDirty = false; router.navigate(pending.route, window, pending.params); }
  }

  async function saveAndNavigate(): Promise<void> {
    if (!(await changeRequest.save()) || changeRequest.dirty) return;
    continueNavigation();
  }

  function discardAndNavigate(): void {
    changeRequest.cancel();
    continueNavigation();
  }

  /**
   * Root: attaches the theme, the router and the backend listeners once, then
   * renders the shell around whichever screen the hash names. The dev harness
   * is a dynamic import behind `import.meta.env.DEV` so a release bundle
   * contains neither its code nor its fixtures.
   *
   * The backend subscriptions resolve asynchronously, so the teardown has to
   * survive unmounting before they land — hence the flag rather than a bare
   * `unlisten?.()`, which would leak a listener into the next mount.
   */
  onMount(() => {
    const detachTheme = theme.attach();
    const detachRouter = router.attach();
    const detachNavigationGuard = setupNavigationGuard();
    const detachPreviewRevocations = previewRevocations.attach();
    const detachTrayAppearance = backendAvailable()
      ? watchForcedColors(setTrayForcedColors)
      : () => {};

    // Settings are loaded at boot rather than when the Settings screen is
    // first opened, because the theme lives in them: waiting would mean the
    // window paints with the mirror's value and then changes under the user.
    if (backendAvailable()) {
      void settings.load();
    }

    let live = true;
    const teardowns: Unlisten[] = [];
    const pending: Promise<Unlisten>[] = [
      monitoring.attach(),
      ...(backendAvailable() ? [collectors.attach()] : []),
      onTrayNavigate((route) => {
        const target = routeById(route as RouteId);
        if (target) {
          router.navigate(target);
        }
      }),
    ];
    for (const subscription of pending) {
      void subscription
        .then((unlisten) => {
          let detached = false;
          const detach = () => {
            if (detached) return;
            detached = true;
            try {
              unlisten();
            } catch {
              // The native window may already be gone during teardown.
            }
          };
          if (live) teardowns.push(detach);
          else detach();
        })
        .catch(() => {
          // One optional event channel must not reject the root mount. Stores
          // either reconcile independently or render their fail-closed state.
        });
    }

    return () => {
      live = false;
      for (const detach of teardowns.splice(0)) detach();
      detachTheme();
      detachRouter();
      detachNavigationGuard();
      detachPreviewRevocations();
      detachTrayAppearance();
    };
  });
</script>

<AppShell>
  <PreviewRevocations />
  {#if !monitoring.resolved && !ownerRoute}
    <div class="flex min-h-0 flex-1 items-center justify-center px-6 py-8" role="status">
      <p class="text-sm text-text-muted">Checking local history…</p>
    </div>
  {:else if monitoring.needsRecovery && router.current.id !== 'diagnostics' && !ownerRoute}
    <RecoverySurface />
  {:else if backendAvailable() && settings.current?.onboarding_completed === false && !ownerRoute}
    <OnboardingFlow oncomplete={() => { void settings.load(); router.navigate(routeById('home')!); }} />
  {:else if import.meta.env.DEV && router.current.id === 'dev-states'}
    {#await import('../features/dev/DevStatesHarness.svelte') then mod}
      <mod.default />
    {/await}
  {:else if import.meta.env.DEV && router.current.id === 'dev-perf'}
    {#await import('../features/dev/PerfHarness.svelte') then mod}
      <mod.default />
    {/await}
  {:else if import.meta.env.DEV && router.current.id === 'dev-system-map-perf'}
    {#await import('../features/dev/SystemMapPerfHarness.svelte') then mod}
      <mod.default />
    {/await}
  {:else if router.current.id === 'owner-apps'}
    <MyAppsScreen onOpenApp={(id) => router.navigate(routeById('owner-app')!, window, { appId: id })} />
  {:else if router.current.id === 'owner-app'}
    <AppOverviewScreen
      appId={router.params.appId}
      legacyProjects={projects.rows}
      onRequestChange={(id) => router.navigate(routeById('owner-change')!, window, { appId: id })}
      onOpenChange={(id, changeId) => router.navigate(routeById('owner-change')!, window, { appId: id, changeId })}
    />
  {:else if router.current.id === 'owner-change'}
    <ChangeRequestScreen
      appId={router.params.appId}
      changeId={router.params.changeId}
      onDirtyChange={(dirty) => (ownerChangeDirty = dirty)}
      onSaved={(appId, changeId) => { if (!changeRequest.dirty) { ownerChangeDirty = false; router.replace(routeById('owner-change')!, { appId, changeId }); } }}
      onLeave={() => { const id = router.params.appId; if (id) router.navigate(routeById('owner-app')!, window, { appId: id }); }}
    />
  {:else if router.current.id !== 'dev-states' && router.current.id !== 'dev-perf' && router.current.id !== 'dev-system-map-perf'}
    {@const Screen = navScreens[router.current.id]}
    {#key router.current.id}
      <Screen />
    {/key}
  {/if}
</AppShell>

<Dialog bind:open={() => pendingNavigation !== null, (open) => { if (!open) pendingNavigation = null; }} title="Leave with unsaved changes?" description="Your change request has wording that is not saved locally.">
  <p class="text-sm text-text-secondary">Save it as a draft before leaving, keep editing, or discard these edits.</p>
  {#snippet footer()}<Button variant="ghost" onclick={() => (pendingNavigation = null)}>Keep editing</Button><Button variant="danger" onclick={discardAndNavigate}>Discard edits</Button><Button variant="primary" disabled={changeRequest.saving || changeRequest.conflict} onclick={() => void saveAndNavigate()}>{changeRequest.saving ? 'Saving…' : 'Save and leave'}</Button>{/snippet}
</Dialog>
