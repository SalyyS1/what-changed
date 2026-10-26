<script lang="ts">
  import { onMount } from 'svelte';
  import { theme } from '../lib/design/theme.svelte';
  import { collectors } from '../lib/stores/collectors.svelte';
  import { monitoring } from '../lib/stores/monitoring.svelte';
  import { router } from '../lib/stores/router.svelte';
  import { settings } from '../lib/stores/settings.svelte';
  import { backendAvailable, setTrayForcedColors } from '../lib/tauri/client';
  import { onTrayNavigate, type Unlisten } from '../lib/tauri/events';
  import { watchForcedColors } from '../lib/tauri/tray-appearance';
  import AppShell from './AppShell.svelte';
  import RecoverySurface from '../features/home/RecoverySurface.svelte';
  import OnboardingFlow from '../features/onboarding/OnboardingFlow.svelte';
  import { routeById, type RouteId } from './routes';
  import { navScreens } from './screens';

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
      detachTrayAppearance();
    };
  });
</script>

<AppShell>
  {#if !monitoring.resolved}
    <div class="flex min-h-0 flex-1 items-center justify-center px-6 py-8" role="status">
      <p class="text-sm text-text-muted">Checking local history…</p>
    </div>
  {:else if monitoring.needsRecovery && router.current.id !== 'diagnostics'}
    <RecoverySurface />
  {:else if backendAvailable() && settings.current?.onboarding_completed === false}
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
  {:else if router.current.id !== 'dev-states' && router.current.id !== 'dev-perf' && router.current.id !== 'dev-system-map-perf'}
    {@const Screen = navScreens[router.current.id]}
    {#key router.current.id}
      <Screen />
    {/key}
  {/if}
</AppShell>
