import { describe, expect, it, vi } from 'vitest';
import { fireEvent, render } from '@testing-library/svelte';
import { createRawSnippet, type Component, type Snippet } from 'svelte';
import StateShell from './StateShell.svelte';
import { empty, failed, loading, ready, type AsyncState } from '../../lib/types/async-state';

/**
 * `StateShell` is the six-state rule made executable: for each branch of
 * `AsyncState<T>` exactly one face renders, and the other three do not. The
 * assertions are on testids and ARIA roles, which is what a screen reader
 * and a later screen test will key on.
 */

const children = createRawSnippet<[string[]]>((data) => ({
  render: () => `<ul data-testid="ready-list">${data().map((s) => `<li>${s}</li>`).join('')}</ul>`,
}));

// `StateShell` is generic over its data type. svelte-check instantiates
// `StateShell<string[]>` fine; plain tsc sees `*.svelte` as an ambient
// non-generic type and cannot, so the test names the prop contract it relies
// on and widens the component to it. svelte-check still checks the component
// itself.
interface ShellProps {
  state: AsyncState<string[]>;
  emptyTitle: string;
  children: Snippet<[string[]]>;
}
const Shell = StateShell as unknown as Component<ShellProps>;

function mount(state: AsyncState<string[]>) {
  return render(Shell, { props: { state, emptyTitle: 'Nothing recorded yet', children } });
}

describe('StateShell', () => {
  it('renders only the loading face while loading', () => {
    const { queryByTestId, getByTestId } = mount(loading());
    expect(getByTestId('state-shell').dataset.state).toBe('loading');
    expect(getByTestId('loading-block').getAttribute('aria-busy')).toBe('true');
    expect(queryByTestId('empty-state')).toBeNull();
    expect(queryByTestId('error-state')).toBeNull();
    expect(queryByTestId('ready-list')).toBeNull();
  });

  it('renders the empty face with the title and the reason from the state value', () => {
    const { getByTestId, queryByTestId, getByText } = mount(empty('Recording started a moment ago.'));
    expect(getByTestId('state-shell').dataset.state).toBe('empty');
    expect(getByText('Nothing recorded yet')).toBeTruthy();
    expect(getByText('Recording started a moment ago.')).toBeTruthy();
    expect(queryByTestId('loading-block')).toBeNull();
    expect(queryByTestId('ready-list')).toBeNull();
  });

  it('renders the error face as an alert with the real message and a working retry', async () => {
    const retry = vi.fn();
    const { getByRole, getByText, queryByTestId } = mount(
      failed('database is locked (code 5)', retry),
    );
    expect(getByRole('alert')).toBeTruthy();
    expect(getByText('database is locked (code 5)')).toBeTruthy();
    await fireEvent.click(getByText('Try again'));
    expect(retry).toHaveBeenCalledTimes(1);
    expect(queryByTestId('loading-block')).toBeNull();
    expect(queryByTestId('ready-list')).toBeNull();
  });

  it('omits the retry control when the state carries no retry', () => {
    const { queryByText } = mount(failed('permission denied'));
    expect(queryByText('Try again')).toBeNull();
  });

  it('renders the children snippet with the data when ready', () => {
    const { getByTestId, queryByTestId } = mount(ready(['first', 'second']));
    expect(getByTestId('state-shell').dataset.state).toBe('ready');
    expect(getByTestId('ready-list').querySelectorAll('li').length).toBe(2);
    expect(queryByTestId('loading-block')).toBeNull();
    expect(queryByTestId('empty-state')).toBeNull();
    expect(queryByTestId('error-state')).toBeNull();
  });
});
