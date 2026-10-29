<script lang="ts">
  import { onMount } from 'svelte';
  import ScreenFrame from '../../components/layout/ScreenFrame.svelte';
  import StateShell from '../../components/state/StateShell.svelte';
  import Badge from '../../components/ui/Badge.svelte';
  import Button from '../../components/ui/Button.svelte';
  import Card from '../../components/ui/Card.svelte';
  import Select from '../../components/ui/Select.svelte';
  import { theme, type ThemeMode } from '../../lib/design/theme.svelte';
  import { formatClock } from '../../lib/formatting/time';
  import { collectors } from '../../lib/stores/collectors.svelte';
  import { monitoring } from '../../lib/stores/monitoring.svelte';
  import { settings } from '../../lib/stores/settings.svelte';
  import { ui } from '../../lib/stores/ui.svelte';
  import type { CollectorStatusDto, MonitoringProfile, Theme, VisualizationMode } from '../../lib/tauri/bindings';
  import { backendAvailable, checkForUpdates, getQuietHours, reopenOnboarding, setQuietHours } from '../../lib/tauri/client';
  import type { AsyncState } from '../../lib/types/async-state';
  import CollectorList from './CollectorList.svelte';
  import WatchedFolders from './WatchedFolders.svelte';
  import Projects from './Projects.svelte';
  import PrivacyScreen from '../privacy/PrivacyScreen.svelte';

  /**
   * Settings. Appearance is live and mirrors into Rust, so the choice survives
   * a restart; the theme still applies immediately from the local store,
   * because waiting for a round trip to repaint would be visible.
   *
   * Collector toggles render from the backend's own list, with the copy that
   * says what each one watches. A toggle that fails to save says so rather
   * than leaving a control showing a state the machine is not in.
   */
  interface Props {
    /** The dev harness drives every face through this; the app leaves it unset. */
    data?: AsyncState<readonly CollectorStatusDto[]>;
  }

  let { data }: Props = $props();

  const live = $derived(data === undefined && backendAvailable());

  onMount(() => {
    if (data !== undefined || !backendAvailable()) {
      return;
    }
    // `App.svelte` already loaded them at boot; this refresh catches a change
    // made from the tray while the screen was closed.
    void settings.load();
    void loadQuietHours();
    // App.svelte owns the live collector subscription because the titlebar
    // consumes health globally. A fresh read here catches changes made while
    // this screen was closed without registering a duplicate listener.
    void collectors.load();
  });

  /** The rows to render: the harness's fixture, or the backend's list. */
  const rows = $derived(data ?? collectors.state);

  const themeOptions = [
    { value: 'dark', label: 'Dark' },
    { value: 'light', label: 'Light' },
    { value: 'system', label: 'Follow Windows' },
  ];
  const profileOptions = [
    { value: 'minimal', label: 'Minimal' },
    { value: 'balanced', label: 'Balanced' },
    { value: 'thorough', label: 'Thorough' },
  ];
  const visualizationOptions = [
    { value: 'auto', label: 'Automatic' },
    { value: '3d', label: '3D' },
    { value: '2d', label: '2D' },
  ];
  const profileCopy: Record<MonitoringProfile, string> = {
    minimal: 'Lowest background activity; slower checks for non-urgent changes.',
    balanced: 'Recommended. Timely monitoring with modest background activity.',
    thorough: 'Most frequent checks; uses more background resources.',
  };

  /**
   * What the control shows: the stored value when the backend has answered,
   * the applied one otherwise. The two agree after `settings.load()` hydrates
   * the theme store, so this only differs during the first frame and outside
   * the app — but reading the store means the control cannot silently show
   * something other than what is stored.
   */
  const selectedTheme = $derived(settings.current?.theme ?? theme.mode);
  const selectedVisualization = $derived(settings.current?.visualization ?? 'auto');
  const selectedProfile = $derived(settings.current?.monitoring_profile ?? null);
  let reopeningTour = $state(false);
  let quietEnabled = $state(false);
  let quietStart = $state(22);
  let quietEnd = $state(8);
  let quietSaving = $state(false);
  let updateOptIn = $state(false);
  let updateChecking = $state(false);
  let updateMessage = $state('Updates are opt-in. No check runs until you ask.');

  const hourOptions = Array.from({ length: 24 }, (_, hour) => ({ value: String(hour), label: `${String(hour).padStart(2, '0')}:00` }));

  async function loadQuietHours(): Promise<void> {
    try {
      const saved = await getQuietHours();
      quietEnabled = saved.enabled;
      quietStart = saved.start_hour;
      quietEnd = saved.end_hour;
    } catch {
      // The notification policy remains fail-closed in the backend. Keep the
      // documented defaults visible if an older/corrupt settings row cannot be
      // read, without turning Settings into an error surface.
    }
  }

  async function chooseTheme(mode: string): Promise<void> {
    // Applied locally first: the repaint is the feedback. A value that failed
    // to store is corrected by the next load rather than by a frozen control.
    theme.set(mode as ThemeMode);
    if (live && !(await settings.save({ theme: mode as Theme }))) {
      ui.toast('That preference could not be saved.', 'warning');
    }
  }

  async function chooseVisualization(mode: string): Promise<void> {
    if (live && !(await settings.save({ visualization: mode as VisualizationMode }))) {
      ui.toast('That map preference could not be saved.', 'warning');
    }
  }

  async function toggleCollector(id: string, enabled: boolean): Promise<void> {
    if (live && !(await collectors.setEnabled(id, enabled))) {
      ui.toast('That collector could not be changed.', 'warning');
    }
  }

  async function chooseProfile(profile: string): Promise<void> {
    if (live && !(await settings.save({ monitoring_profile: profile as MonitoringProfile }))) {
      ui.toast('That monitoring profile could not be saved.', 'warning');
    }
  }

  async function saveQuietHours(): Promise<void> {
    if (!live) return;
    quietSaving = true;
    try {
      const saved = await setQuietHours({ enabled: quietEnabled, start_hour: quietStart, end_hour: quietEnd });
      quietEnabled = saved.enabled;
      quietStart = saved.start_hour;
      quietEnd = saved.end_hour;
    } catch {
      ui.toast('Quiet hours could not be saved.', 'warning');
    } finally {
      quietSaving = false;
    }
  }

  async function checkUpdates(): Promise<void> {
    if (!live || !updateOptIn || updateChecking) return;
    updateChecking = true;
    try {
      const result = await checkForUpdates(true);
      updateMessage = result.message;
    } catch {
      updateMessage = 'The update check could not be started.';
    } finally {
      updateChecking = false;
    }
  }

  async function toggleMonitoring(): Promise<void> {
    if (!(await monitoring.setPaused(!monitoring.paused))) {
      ui.toast(`Monitoring could not be ${monitoring.paused ? 'resumed' : 'paused'}.`, 'warning');
    }
  }

  async function showWelcomeTourAgain(): Promise<void> {
    if (!live || reopeningTour) return;
    reopeningTour = true;
    try {
      await reopenOnboarding();
      await settings.load();
    } catch {
      ui.toast('The welcome tour could not be reopened.', 'warning');
    } finally {
      reopeningTour = false;
    }
  }
</script>

<ScreenFrame title="Settings">
  <section class="mb-6">
    <h2 class="mb-2 text-caps text-text-muted">Appearance</h2>
    <Card level={1}>
      <div class="flex items-center justify-between gap-4">
        <div>
          <p class="text-base text-text-primary">Theme</p>
          <p class="text-sm text-text-secondary">Dark is the default; light is complete.</p>
        </div>
        <Select
          options={themeOptions}
          value={selectedTheme}
          label="Theme"
          class="w-44"
          onchange={(v) => void chooseTheme(v)}
        />
      </div>
      <div class="mt-4 flex items-center justify-between gap-4 border-t border-border-subtle pt-4">
        <div>
          <p class="text-base text-text-primary">Change Map</p>
          <p class="text-sm text-text-secondary">Automatic uses 2D for reduced motion or graphics fallback.</p>
        </div>
        <Select
          options={visualizationOptions}
          value={selectedVisualization}
          label="Change Map visualization"
          class="w-44"
          onchange={(value) => void chooseVisualization(value)}
        />
      </div>
    </Card>
  </section>

  <section class="mb-6">
    <h2 class="mb-2 text-caps text-text-muted">Updates</h2>
    <Card level={1}>
      <div class="flex flex-col gap-4">
        <div>
          <p class="text-base text-text-primary">Check for updates</p>
          <p class="mt-1 text-sm text-text-secondary">Only runs after you enable this option and press the button. This preview has no release feed or signing key yet.</p>
        </div>
        <label class="flex items-start gap-3 text-sm text-text-primary">
          <input class="mt-1 size-4" type="checkbox" bind:checked={updateOptIn} disabled={!live || updateChecking} />
          <span><strong>Ask for update checks</strong><span class="mt-1 block text-xs text-text-muted">No background or automatic checks.</span></span>
        </label>
        <div class="flex flex-wrap items-center gap-3">
          <Button variant="secondary" disabled={!live || !updateOptIn || updateChecking} onclick={() => void checkUpdates()}>
            {updateChecking ? 'Checking…' : 'Check now'}
          </Button>
          <p class="text-xs text-text-muted" role="status">{updateMessage}</p>
        </div>
      </div>
    </Card>
  </section>

  <section class="mb-6">
    <h2 class="mb-2 text-caps text-text-muted">Notifications</h2>
    <Card level={1}>
      <div class="flex flex-col gap-4">
        <label class="flex items-start gap-3 text-sm text-text-primary">
          <input class="mt-1 size-4" type="checkbox" bind:checked={quietEnabled} disabled={!live || quietSaving} onchange={() => void saveQuietHours()} />
          <span><strong>Quiet hours</strong><span class="mt-1 block text-xs text-text-muted">Suppress native notifications during the local clock window.</span></span>
        </label>
        <div class="flex flex-wrap items-end gap-3">
          <Select options={hourOptions} value={String(quietStart)} label="Quiet hours start" disabled={!live || quietSaving} onchange={(value) => { quietStart = Number(value); void saveQuietHours(); }} />
          <Select options={hourOptions} value={String(quietEnd)} label="Quiet hours end" disabled={!live || quietSaving} onchange={(value) => { quietEnd = Number(value); void saveQuietHours(); }} />
        </div>
      </div>
    </Card>
  </section>

  <section class="mb-6">
    <h2 class="mb-2 text-caps text-text-muted">Monitoring</h2>
    <Card level={1}>
      <div class="flex flex-col gap-4 @[680px]/shell:flex-row @[680px]/shell:items-start @[680px]/shell:justify-between">
        <div class="min-w-0">
          <div class="flex items-center gap-2">
            <p class="text-base text-text-primary">Collection status</p>
            <Badge tone={!monitoring.resolved ? 'neutral' : monitoring.paused ? 'warning' : 'success'} dot>{!monitoring.resolved ? 'Starting' : monitoring.paused ? 'Paused' : 'Running'}</Badge>
          </div>
          <p class="mt-1 text-sm text-text-secondary">
            {!monitoring.resolved
              ? 'Checking the recorder status…'
              : monitoring.paused
              ? monitoring.resumeAt === null ? 'No new changes are being recorded.' : `Recording resumes at ${formatClock(monitoring.resumeAt)}.`
              : 'Changes are being recorded locally.'}
          </p>
          {#if monitoring.error}<p class="mt-2 text-xs text-danger" role="alert">{monitoring.error}</p>{/if}
        </div>
        <Button disabled={!live || !monitoring.resolved || monitoring.pending} onclick={() => void toggleMonitoring()}>
          {!monitoring.resolved ? 'Checking…' : monitoring.pending ? 'Updating…' : monitoring.paused ? 'Resume monitoring' : 'Pause monitoring'}
        </Button>
      </div>
      <div class="mt-4 border-t border-border-subtle pt-4">
        <div class="flex flex-col gap-3 @[680px]/shell:flex-row @[680px]/shell:items-start @[680px]/shell:justify-between">
          <div>
            <p class="text-base text-text-primary">Monitoring profile</p>
            <p class="mt-1 text-sm text-text-secondary">{selectedProfile === null ? 'Loading the saved profile…' : profileCopy[selectedProfile]}</p>
          </div>
          <Select
            options={profileOptions}
            value={selectedProfile ?? ''}
            label="Monitoring profile"
            placeholder="Loading profile…"
            disabled={!live || settings.saving || selectedProfile === null}
            class="w-full @[680px]/shell:w-44"
            onchange={(value) => void chooseProfile(value)}
          />
        </div>
      </div>
    </Card>
  </section>

  <section class="mb-6">
    <h2 class="mb-2 text-caps text-text-muted">What is watched</h2>
    <StateShell
      state={rows}
      emptyTitle="Collectors are not available yet."
      emptyDescription="Settings load once the local service is running."
      emptyIcon="settings"
      loadingRows={4}
    >
      {#snippet children(list)}
        <CollectorList
          rows={list}
          disabled={!live || collectors.pending.size > 0}
          pending={collectors.pending}
          errors={collectors.errors}
          ontoggle={(id, enabled) => void toggleCollector(id, enabled)}
        />
      {/snippet}
    </StateShell>
  </section>

  <section class="mb-6">
    <h2 class="mb-2 text-caps text-text-muted">Watched folders</h2>
    <WatchedFolders active={live} />
  </section>

  <section class="mb-6">
    <h2 class="mb-2 text-caps text-text-muted">Workspaces</h2>
    <Projects active={live} />
  </section>

  <section>
    <h2 class="mb-2 text-caps text-text-muted">Privacy</h2>
    <Card level={1} class="mb-3">
      <div class="flex flex-wrap items-center justify-between gap-3">
        <div>
          <p class="m-0 text-base text-text-primary">Welcome tour</p>
          <p class="m-0 mt-1 text-sm text-text-secondary">Review what is watched and choose folders again.</p>
        </div>
        <Button variant="secondary" disabled={!live || reopeningTour} onclick={() => void showWelcomeTourAgain()}>
          {reopeningTour ? 'Opening…' : 'Show welcome tour again'}
        </Button>
      </div>
    </Card>
    <PrivacyScreen active={live} />
  </section>
</ScreenFrame>
