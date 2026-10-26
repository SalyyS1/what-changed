import { beforeEach, describe, expect, it, vi } from 'vitest';

const { open } = vi.hoisted(() => ({ open: vi.fn() }));

vi.mock('@tauri-apps/plugin-dialog', () => ({ open }));

import { pickDirectory } from './dialog';

describe('pickDirectory', () => {
  beforeEach(() => open.mockReset());

  it('opens a single directory-only native picker', async () => {
    open.mockResolvedValue('C:\\code\\api');

    await expect(pickDirectory()).resolves.toBe('C:\\code\\api');
    expect(open).toHaveBeenCalledWith({
      directory: true,
      multiple: false,
      title: 'Choose a folder to watch',
    });
  });

  it('treats cancellation as a silent null result', async () => {
    open.mockResolvedValue(null);
    await expect(pickDirectory()).resolves.toBeNull();
  });
});
