import type { PrivacySummaryDto } from '../tauri/bindings';
import {
  getPrivacySummary,
  purgeAll,
  purgeCategory,
  requestPurgeChallenge,
  toAppError,
} from '../tauri/client';
import { failed, loading, ready, type AsyncState } from '../types/async-state';
import { settings } from './settings.svelte';

export class PrivacyStore {
  state = $state<AsyncState<PrivacySummaryDto>>(loading());
  pending = $state(false);
  error = $state<string | null>(null);

  get summary(): PrivacySummaryDto | null {
    return this.state.status === 'ready' ? this.state.data : null;
  }

  async load(): Promise<void> {
    try {
      const value = await getPrivacySummary();
      this.state = ready(value);
      this.error = null;
    } catch (reason) {
      const error = toAppError(reason);
      this.state = failed(error.message, error.retryable ? () => void this.load() : undefined);
      this.error = error.message;
    }
  }

  async removeCategory(category: string): Promise<boolean> {
    this.pending = true;
    try {
      await purgeCategory(category);
      await this.load();
      return true;
    } catch (reason) {
      this.error = toAppError(reason).message;
      return false;
    } finally {
      this.pending = false;
    }
  }

  async removeEverything(): Promise<boolean> {
    this.pending = true;
    try {
      const challenge = await requestPurgeChallenge();
      await purgeAll('delete', challenge);
      await this.load();
      await settings.load();
      return true;
    } catch (reason) {
      this.error = toAppError(reason).message;
      return false;
    } finally {
      this.pending = false;
    }
  }
}

export const privacy = new PrivacyStore();
