import { fixtureEvidenceReview, fixtureScenario } from '../../features/acceptance/owner-evidence-fixtures';
import { fixtureRule, fixtureDraft } from '../../features/owner-workspace/owner-workspace-fixtures';
import type { OwnerGraphInput } from './types';

/** Synthetic graph inputs for focused tests only. Never imported by production. */
export function ownerGraphFixture(): OwnerGraphInput {
  return { appId: 'app-a', capabilities: [{ id: 'capability-a', app_id: 'app-a', label: 'Bookings', description: 'Booking behavior', version: 1, sort_order: 0, archived_at: null }],
    rules: [fixtureRule()], scenarios: [fixtureScenario()], reviews: [fixtureEvidenceReview()], change: fixtureDraft() };
}
