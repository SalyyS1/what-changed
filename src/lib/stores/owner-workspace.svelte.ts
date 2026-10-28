import { empty, failed, loading, ready, type AsyncState } from '../types/async-state';
import type {
  OwnerAppDto, OwnerCapabilityDto, OwnerRuleRevisionDto, OwnerChangeDraftDto,
  OwnerEnvironmentRevisionDto, OwnerCandidateRevisionDto, OwnerPageInput, OwnerPageDto,
  SaveOwnerAppInput, SaveOwnerCapabilityInput, ConfirmOwnerRuleInput,
  SaveOwnerEnvironmentInput, SaveOwnerCandidateInput, SelectOwnerCandidateInput,
} from '../tauri/bindings';
import {
  listOwnerApps, saveOwnerApp, getOwnerApp, saveOwnerCapability, listOwnerCapabilities,
  confirmOwnerRule, listOwnerRuleRevisions, listOwnerChangeDrafts, saveOwnerEnvironment,
  listOwnerEnvironments, saveOwnerCandidate, listOwnerCandidates, selectOwnerCandidate, toAppError,
} from '../tauri/client';

export interface OwnerWorkspaceDetail {
  app: OwnerAppDto;
  capabilities: OwnerCapabilityDto[];
  rules: OwnerRuleRevisionDto[];
  changes: OwnerChangeDraftDto[];
  environments: OwnerEnvironmentRevisionDto[];
  candidates: OwnerCandidateRevisionDto[];
}

export interface OwnerWorkspaceDependencies {
  listApps: typeof listOwnerApps;
  saveApp: typeof saveOwnerApp;
  getApp: typeof getOwnerApp;
  saveCapability: typeof saveOwnerCapability;
  listCapabilities: typeof listOwnerCapabilities;
  confirmRule: typeof confirmOwnerRule;
  listRules: typeof listOwnerRuleRevisions;
  listChanges: typeof listOwnerChangeDrafts;
  saveEnvironment: typeof saveOwnerEnvironment;
  listEnvironments: typeof listOwnerEnvironments;
  saveCandidate: typeof saveOwnerCandidate;
  listCandidates: typeof listOwnerCandidates;
  selectCandidate: typeof selectOwnerCandidate;
  uuid: () => string;
}

const defaults: OwnerWorkspaceDependencies = {
  listApps: listOwnerApps, saveApp: saveOwnerApp, getApp: getOwnerApp,
  saveCapability: saveOwnerCapability, listCapabilities: listOwnerCapabilities,
  confirmRule: confirmOwnerRule, listRules: listOwnerRuleRevisions,
  listChanges: listOwnerChangeDrafts, saveEnvironment: saveOwnerEnvironment,
  listEnvironments: listOwnerEnvironments, saveCandidate: saveOwnerCandidate,
  listCandidates: listOwnerCandidates, selectCandidate: selectOwnerCandidate,
  uuid: () => crypto.randomUUID(),
};

/** Read all pages so a saved rule is never silently omitted from a later request. */
export async function ownerPages<T>(read: (page: OwnerPageInput) => Promise<OwnerPageDto<T>>): Promise<T[]> {
  const items: T[] = [];
  let after: string | null = null;
  const cursors = new Set<string>();
  do {
    const page = await read({ after, limit: 100 });
    items.push(...page.items);
    after = page.next_cursor;
    if (after !== null && cursors.has(after)) throw { code: 'database', message: 'The saved list could not be completed. Try loading it again.', retryable: true };
    if (after !== null) cursors.add(after);
  } while (after !== null);
  return items;
}

export function latestOwnerRules(rules: OwnerRuleRevisionDto[]): OwnerRuleRevisionDto[] {
  const latest = new Map<string, OwnerRuleRevisionDto>();
  for (const rule of rules) {
    if ((latest.get(rule.rule_id)?.revision ?? 0) < rule.revision) latest.set(rule.rule_id, rule);
  }
  return [...latest.values()];
}

type AppFields = Pick<SaveOwnerAppInput, 'name' | 'description' | 'legacy_project_id' | 'archived'>;
type CapabilityFields = Pick<SaveOwnerCapabilityInput, 'label' | 'description'>;
type EnvironmentFields = Omit<SaveOwnerEnvironmentInput, 'operation_id' | 'id' | 'app_id' | 'expected_version'>;
type CandidateFields = Omit<SaveOwnerCandidateInput, 'operation_id' | 'id' | 'app_id'>;

export class OwnerWorkspaceStore {
  apps = $state<AsyncState<OwnerAppDto[]>>(loading());
  detail = $state<AsyncState<OwnerWorkspaceDetail>>(loading());
  appId = $state<string | null>(null);
  includeArchived = $state(false);
  saving = $state(false);
  error = $state<string | null>(null);
  #deps: OwnerWorkspaceDependencies;
  #listGeneration = 0;
  #generation = 0;

  constructor(deps: Partial<OwnerWorkspaceDependencies> = {}) { this.#deps = { ...defaults, ...deps }; }
  get rows(): OwnerAppDto[] { return this.apps.status === 'ready' ? this.apps.data : []; }
  get current(): OwnerWorkspaceDetail | null { return this.detail.status === 'ready' ? this.detail.data : null; }

  cancel(): void {
    ++this.#generation;
    ++this.#listGeneration;
    this.appId = null;
    this.detail = loading();
    this.saving = false;
    this.error = null;
  }

  async loadApps(includeArchived = this.includeArchived): Promise<void> {
    const generation = ++this.#listGeneration;
    this.includeArchived = includeArchived;
    this.apps = loading();
    this.error = null;
    try {
      const rows = await this.#deps.listApps(includeArchived);
      if (generation !== this.#listGeneration) return;
      this.apps = rows.length ? ready(rows) : empty();
    } catch (reason) {
      if (generation === this.#listGeneration) this.apps = failed(toAppError(reason).message, () => void this.loadApps(includeArchived));
    }
  }

  async loadApp(appId: string): Promise<void> {
    const generation = ++this.#generation;
    this.appId = appId;
    this.detail = loading();
    this.error = null;
    this.saving = false;
    try {
      const [app, capabilities, rules, changes, environments, candidates] = await Promise.all([
        this.#deps.getApp(appId), this.#deps.listCapabilities(appId, false),
        ownerPages((page) => this.#deps.listRules(appId, page)),
        ownerPages((page) => this.#deps.listChanges(appId, page)),
        ownerPages((page) => this.#deps.listEnvironments(appId, page)),
        ownerPages((page) => this.#deps.listCandidates(appId, page)),
      ]);
      if (this.#matches(appId, generation)) this.detail = ready({ app, capabilities, rules, changes, environments, candidates });
    } catch (reason) {
      if (this.#matches(appId, generation)) this.detail = failed(toAppError(reason).message, () => void this.loadApp(appId));
    }
  }

  async saveApp(fields: AppFields, previous?: OwnerAppDto): Promise<OwnerAppDto | null> {
    if (this.saving) return null;
    const generation = this.#generation;
    const listGeneration = this.#listGeneration;
    const appId = this.appId;
    this.saving = true;
    this.error = null;
    try {
      const app = await this.#deps.saveApp({ ...fields, operation_id: this.#deps.uuid(), id: previous?.id ?? this.#deps.uuid(), expected_version: previous?.version ?? 0 });
      if (generation !== this.#generation || listGeneration !== this.#listGeneration || appId !== this.appId) return null;
      const rows = this.rows.filter((row) => row.id !== app.id);
      if (this.includeArchived || app.archived_at === null) rows.unshift(app);
      this.apps = rows.length ? ready(rows) : empty();
      const current = this.current;
      if (current?.app.id === app.id) this.detail = ready({ ...current, app });
      return app;
    } catch (reason) {
      if (generation === this.#generation && listGeneration === this.#listGeneration) this.error = toAppError(reason).message;
      return null;
    } finally {
      if (generation === this.#generation) this.saving = false;
    }
  }

  saveCapability(fields: CapabilityFields, previous?: OwnerCapabilityDto): Promise<boolean> {
    return this.#mutate((appId) => this.#deps.saveCapability({ ...fields, operation_id: this.#deps.uuid(), id: previous?.id ?? this.#deps.uuid(), app_id: appId, expected_version: previous?.version ?? 0, sort_order: previous?.sort_order ?? this.current!.capabilities.length, archived: false }));
  }

  confirmRule(capabilityId: string, expectedBehavior: string, previous?: OwnerRuleRevisionDto): Promise<boolean> {
    return this.#mutate((appId) => this.#deps.confirmRule({ operation_id: this.#deps.uuid(), rule_id: previous?.rule_id ?? this.#deps.uuid(), app_id: appId, capability_id: capabilityId, expected_version: previous?.revision ?? 0, expected_behavior: expectedBehavior, provenance: 'owner', confirmed_by: 'App owner' } satisfies ConfirmOwnerRuleInput));
  }

  saveEnvironment(fields: EnvironmentFields, previous?: OwnerEnvironmentRevisionDto): Promise<boolean> {
    return this.#mutate((appId) => this.#deps.saveEnvironment({ ...fields, operation_id: this.#deps.uuid(), id: previous?.id ?? this.#deps.uuid(), app_id: appId, expected_version: previous?.revision ?? 0 }));
  }

  saveCandidate(fields: CandidateFields): Promise<boolean> {
    return this.#mutate((appId) => this.#deps.saveCandidate({ ...fields, operation_id: this.#deps.uuid(), id: this.#deps.uuid(), app_id: appId }));
  }

  selectCandidate(candidateId: string): Promise<boolean> {
    return this.#mutate((appId) => this.#deps.selectCandidate({ operation_id: this.#deps.uuid(), app_id: appId, expected_version: this.current!.app.version, candidate_id: candidateId } satisfies SelectOwnerCandidateInput));
  }

  #matches(appId: string, generation: number): boolean { return appId === this.appId && generation === this.#generation; }

  async #mutate(write: (appId: string) => Promise<unknown>): Promise<boolean> {
    const appId = this.appId;
    const generation = this.#generation;
    if (!appId || !this.current || this.saving || this.current.app.archived_at !== null) return false;
    this.saving = true;
    this.error = null;
    try {
      await write(appId);
      if (!this.#matches(appId, generation)) return false;
      await this.loadApp(appId);
      return this.appId === appId && this.detail.status === 'ready';
    } catch (reason) {
      if (this.#matches(appId, generation)) this.error = toAppError(reason).message;
      return false;
    } finally {
      if (this.#matches(appId, generation)) this.saving = false;
    }
  }
}

export const ownerWorkspace = new OwnerWorkspaceStore();
