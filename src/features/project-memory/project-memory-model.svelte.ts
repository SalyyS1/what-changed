import type { JsonValue, ProjectDecisionRevisionDto, SaveProjectDecisionInput } from '../../lib/tauri/bindings';
import { getImportedOwnerHistory, listProjectDecisions, saveProjectDecision, toAppError } from '../../lib/tauri/client';

export const memoryProvenance = { owner: 'Owner wording', template: 'Template', external_ai: 'External AI suggestion', imported_untrusted: 'Imported, untrusted' };
export type DecisionDraft = Pick<SaveProjectDecisionInput, 'capability_id' | 'change_id' | 'text' | 'rationale' | 'provenance' | 'recorded_by'>;
interface Dependencies { list: typeof listProjectDecisions; save: typeof saveProjectDecision; history: typeof getImportedOwnerHistory; }
const defaults: Dependencies = { list: listProjectDecisions, save: saveProjectDecision, history: getImportedOwnerHistory };

export function latestDecisions(revisions: ProjectDecisionRevisionDto[]): ProjectDecisionRevisionDto[] {
  const latest = new Map<string, ProjectDecisionRevisionDto>();
  for (const revision of revisions) if ((latest.get(revision.id)?.revision ?? 0) < revision.revision) latest.set(revision.id, revision);
  return [...latest.values()];
}

/** Memory is owner-authored context; it never changes a test result or acceptance. */
export class ProjectMemoryModel {
  appId = $state<string | null>(null);
  revisions = $state<ProjectDecisionRevisionDto[]>([]);
  loading = $state(false);
  saving = $state(false);
  error = $state<string | null>(null);
  history = $state<JsonValue>(null);
  historyLoaded = $state(false);
  historyLoading = $state(false);
  historyError = $state<string | null>(null);
  #generation = 0;
  #load = 0;
  #deps: Dependencies;
  constructor(deps: Partial<Dependencies> = {}) { this.#deps = { ...defaults, ...deps }; }

  bind(appId: string): void { if (this.appId === appId) return; this.cancel(); this.appId = appId; void this.load(); }
  cancel(): void {
    ++this.#generation; ++this.#load; this.appId = null; this.revisions = [];
    this.loading = this.saving = this.historyLoading = this.historyLoaded = false;
    this.error = this.historyError = null; this.history = null;
  }
  async load(): Promise<void> {
    const appId = this.appId; if (!appId) return;
    const generation = this.#generation; const load = ++this.#load;
    this.loading = true; this.error = null;
    try {
      const revisions: ProjectDecisionRevisionDto[] = []; const cursors = new Set<string>(); let after: string | null = null;
      do {
        const page = await this.#deps.list(appId, { capability_id: null, change_id: null, page: { after, limit: 100 } });
        if (!this.#matches(appId, generation) || load !== this.#load) return;
        if (page.items.some((item) => item.app_id !== appId)) throw new Error('The decision history did not match this app. Reload before continuing.');
        revisions.push(...page.items); after = page.next_cursor;
        if (after && cursors.has(after)) throw new Error('The decision history could not be fully loaded. Reload before continuing.');
        if (after) cursors.add(after);
      } while (after);
      this.revisions = revisions;
    } catch (reason) { if (this.#matches(appId, generation) && load === this.#load) this.error = toAppError(reason).message; }
    finally { if (this.#matches(appId, generation) && load === this.#load) this.loading = false; }
  }
  async save(draft: DecisionDraft, previous?: ProjectDecisionRevisionDto): Promise<boolean> {
    const appId = this.appId;
    if (!appId || this.saving || (previous && previous.app_id !== appId)) return false;
    const generation = this.#generation; const id = previous?.id ?? crypto.randomUUID();
    this.saving = true; this.error = null;
    try {
      const saved = await this.#deps.save({ ...draft, operation_id: crypto.randomUUID(), id, app_id: appId, expected_revision: previous?.revision ?? 0 });
      if (!this.#matches(appId, generation)) return false;
      if (saved.app_id !== appId || saved.id !== id || saved.revision !== (previous?.revision ?? 0) + 1) throw new Error('The saved revision did not match this decision. Reload its history before continuing.');
      this.revisions = [...this.revisions, saved]; return true;
    } catch (reason) {
      if (this.#matches(appId, generation)) this.error = `${toAppError(reason).message} Your wording is still here. Reload the history before retrying a conflicting revision.`;
      return false;
    } finally { if (this.#matches(appId, generation)) this.saving = false; }
  }
  async loadHistory(): Promise<void> {
    const appId = this.appId; if (!appId || this.historyLoading) return;
    const generation = this.#generation; this.historyLoading = true; this.historyError = null;
    try { const history = await this.#deps.history(appId); if (this.#matches(appId, generation)) { this.history = history; this.historyLoaded = true; } }
    catch (reason) { if (this.#matches(appId, generation)) this.historyError = toAppError(reason).message; }
    finally { if (this.#matches(appId, generation)) this.historyLoading = false; }
  }
  #matches(appId: string, generation: number): boolean { return this.appId === appId && this.#generation === generation; }
}
