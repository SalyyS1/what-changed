/**
 * Native dialog calls live behind this seam so features never depend on the
 * plugin directly and tests can replace the picker without opening Windows UI.
 */

import { open } from '@tauri-apps/plugin-dialog';

/** Opens one directory picker. Cancelling is a normal, silent `null` result. */
export async function pickDirectory(title = 'Choose a folder to watch'): Promise<string | null> {
  const selection = await open({
    directory: true,
    multiple: false,
    title,
  });

  return typeof selection === 'string' ? selection : null;
}
