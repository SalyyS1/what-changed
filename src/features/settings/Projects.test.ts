import { render, screen } from '@testing-library/svelte';
import { expect, it, vi } from 'vitest';
import type { Project } from '../../lib/stores/projects.svelte';
import { ProjectsStore } from '../../lib/stores/projects.svelte';
import Projects from './Projects.svelte';

function project(overrides: Partial<Project> = {}): Project {
  return {
    id: 'project:1',
    path: 'C:\\code\\api',
    name: 'api',
    branch: 'feature/recovery',
    baseline_checkpoint_id: null,
    baseline_checkpoint_name: null,
    baseline_checkpoint_created_at: null,
    ...overrides,
  };
}

it('renders a project, branch and baseline action', async () => {
  const model = new ProjectsStore({
    list: vi.fn().mockResolvedValue([project()]),
    add: vi.fn(),
    remove: vi.fn(),
    markWorking: vi.fn(),
    clearBaseline: vi.fn(),
    chooseDirectory: vi.fn(),
  });
  await model.load();
  render(Projects, { active: false, model });

  expect(screen.getByText('api')).toBeTruthy();
  expect(screen.getByText('feature/recovery')).toBeTruthy();
  expect(screen.getByText('Needs baseline')).toBeTruthy();
  expect(screen.getByRole('button', { name: 'Mark working' })).toBeTruthy();
});

it('shows the saved baseline and clear action', async () => {
  const model = new ProjectsStore({
    list: vi.fn().mockResolvedValue([project({
      baseline_checkpoint_id: 9,
      baseline_checkpoint_name: 'Working: api',
      baseline_checkpoint_created_at: 1_800_000_000_000,
    })]),
    add: vi.fn(),
    remove: vi.fn(),
    markWorking: vi.fn(),
    clearBaseline: vi.fn(),
    chooseDirectory: vi.fn(),
  });
  await model.load();
  render(Projects, { active: false, model });

  expect(screen.getByText('Working state saved')).toBeTruthy();
  expect(screen.getByText(/Baseline: Working: api/)).toBeTruthy();
  expect(screen.getByRole('button', { name: 'Clear baseline' })).toBeTruthy();
});
