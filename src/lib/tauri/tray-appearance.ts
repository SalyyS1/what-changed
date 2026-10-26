/**
 * Relay only the OS forced-colors bit to the native tray. The native side owns
 * light/dark selection from its window theme; this listener never polls or
 * lets the WebView choose an icon path.
 */
export function watchForcedColors(
  report: (active: boolean) => Promise<void>,
  matchMedia: ((query: string) => MediaQueryList) | null | undefined = globalThis.matchMedia?.bind(globalThis),
): () => void {
  const media = matchMedia?.('(forced-colors: active)');
  if (!media) return () => {};

  let live = true;
  let running = false;
  let latest = media.matches;

  const drain = async () => {
    if (running) return;
    running = true;
    while (live) {
      const sent = latest;
      try {
        await report(sent);
      } catch {
        // A failed cosmetic update must not reject App's mount or prevent a
        // newer state from being delivered.
      }
      if (latest === sent) break;
    }
    running = false;
  };

  const sync = () => {
    latest = media.matches;
    // IPC completions can arrive out of order when calls overlap. Keep one
    // call in flight and coalesce intermediate changes to the latest bit.
    void drain();
  };
  media.addEventListener('change', sync);
  sync();
  return () => {
    live = false;
    media.removeEventListener('change', sync);
  };
}
