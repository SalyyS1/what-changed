import { render, screen } from '@testing-library/svelte';
import { describe, expect, it, vi } from 'vitest';
import FilterChips from './FilterChips.svelte';

describe('FilterChips responsive contract', () => {
  it('uses the dropdown, not the full chip row, at the 900px minimum shell', () => {
    render(FilterChips, { value: 'all', onchange: vi.fn() });

    const chips = screen.getByTestId('timeline-filter-chips');
    const menu = screen.getByTestId('timeline-filter-menu');
    const breakpoint = Number(menu.dataset.collapseBelow);

    expect(900).toBeLessThan(breakpoint);
    expect(chips.classList.contains('hidden')).toBe(true);
    expect(chips.classList.contains('@[1040px]/shell:flex')).toBe(true);
    expect(menu.classList.contains('hidden')).toBe(false);
    expect(menu.classList.contains('@[1040px]/shell:hidden')).toBe(true);
    expect(screen.getByRole('button', { name: /Filter: All/ })).toBeTruthy();
  });
});
