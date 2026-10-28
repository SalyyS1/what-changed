import type { OwnerChangeReviewDto, RunnerRouteDto, ScenarioRevisionDto } from '../../lib/tauri/bindings';

export function runnerOrigin(origin: { scheme: string; host: string; port: number }): string {
  return new URL(`${origin.scheme}://${origin.host.includes(':') ? `[${origin.host}]` : origin.host}:${origin.port}`).origin;
}

/** Suggestions are visible, editable scope. They are never an execution grant. */
export function suggestedRunnerRoutes(change: OwnerChangeReviewDto, scenarios: ScenarioRevisionDto[]): RunnerRouteDto[] {
  if (!change.environment) return [];
  const preview = new URL(change.environment.preview_url);
  const routes: RunnerRouteDto[] = [
    { origin: preview.origin, method: 'GET', path: preview.pathname },
    { origin: preview.origin, method: 'GET', path: '/.well-known/ownvero.json' },
  ];
  for (const scenario of scenarios) {
    for (const step of scenario.recipe.steps) if (step.action.kind === 'navigate') routes.push({ origin: preview.origin, method: 'GET', path: step.action.path });
    for (const adapter of scenario.recipe.adapters) routes.push({ origin: runnerOrigin(adapter.origin), method: 'GET', path: adapter.path_template });
  }
  return routes.filter((item, index) => routes.findIndex((other) => other.origin === item.origin && other.method === item.method && other.path === item.path) === index);
}

export const runnerFixtureFields = (scenarios: ScenarioRevisionDto[]): string[] => [...new Set(scenarios.flatMap((scenario) => scenario.recipe.fixture_refs))].sort();
