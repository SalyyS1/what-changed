<script lang="ts">
  import { untrack } from 'svelte';
  import { empty, failed, loading, ready } from '../../lib/types/async-state';
  import { OwnerWorkspaceStore } from '../../lib/stores/owner-workspace.svelte';
  import { ChangeRequestStore } from '../../lib/stores/change-request.svelte';
  import { fixtureApp, fixtureDraft, fixtureRule } from './owner-workspace-fixtures';
  import MyAppsScreen from './MyAppsScreen.svelte';
  import AppOverviewScreen from './AppOverviewScreen.svelte';
  import ChangeRequestScreen from '../change-request/ChangeRequestScreen.svelte';

  interface Props { screen?: 'apps' | 'overview' | 'change'; state?: 'loading' | 'empty' | 'error' | 'ready'; }
  let { screen = 'overview', state = 'ready' }: Props = $props();
  const app = { ...fixtureApp('synthetic-booking-app'), name: 'Example booking app' };
  const draft = { ...fixtureDraft(app.id), id: 'synthetic-request' };
  const rule = { ...fixtureRule(), app_id: app.id, capability_id: 'synthetic-capability' };
  const reject = async (): Promise<never> => { throw { code: 'not_ready', message: 'Synthetic example only. No production workspace was opened.', retryable: false }; };
  const workspace = new OwnerWorkspaceStore({ listApps: reject, saveApp: reject, getApp: reject,
    saveCapability: reject, listCapabilities: reject, confirmRule: reject, listRules: reject,
    listChanges: reject, saveEnvironment: reject, listEnvironments: reject, saveCandidate: reject,
    listCandidates: reject, selectCandidate: reject, uuid: () => 'synthetic-example' });
  const request = new ChangeRequestStore({ getApp: async () => app,
    getReview: async () => ({ app, change: draft, preserved_rules: [rule], candidate: null, environment: null }),
    listRules: async () => ({ items: [rule], next_cursor: null }), save: reject, uuid: () => 'synthetic-example' });
  $effect(() => {
    const display = state;
    untrack(() => {
      workspace.appId = app.id;
      workspace.apps = display === 'loading' ? loading() : display === 'error' ? failed('Synthetic: the app memory could not be loaded.') : display === 'empty' ? empty() : ready([app]);
      workspace.detail = display === 'loading' ? loading() : display === 'error' ? failed('Synthetic: the app memory could not be loaded.') : display === 'empty' ? empty() : ready({ app,
        capabilities: [{ id: 'synthetic-capability', app_id: app.id, label: 'Create a booking', description: 'A customer chooses an available time and staff can read it.', sort_order: 0, version: 1, archived_at: null }],
        rules: [rule], changes: [draft], environments: [], candidates: [] });
      request.cancel();
      if (display === 'ready') void request.load(app.id, draft.id);
      else request.state = display === 'loading' ? loading() : display === 'error' ? failed('Synthetic: this request could not be loaded.') : empty();
    });
  });
</script>

<div class="flex min-h-0 flex-1 flex-col" data-testid="synthetic-owner-workspace">
  <p class="border-b border-border-subtle bg-surface-2 px-4 py-2 text-xs text-text-secondary">Synthetic booking example · read only · no app data is saved or loaded · no preview is contacted</p>
  {#if screen === 'apps'}<MyAppsScreen active={false} model={workspace} />
  {:else if screen === 'overview'}<AppOverviewScreen active={false} appId="synthetic-booking-app" model={workspace} />
  {:else}<ChangeRequestScreen active={false} appId="synthetic-booking-app" changeId="synthetic-request" model={request} />{/if}
</div>
