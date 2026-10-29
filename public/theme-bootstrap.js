// Runs before first paint, as a blocking classic script from <head>, so the
// theme is on <html> before any stylesheet applies. A separate file rather
// than inline because the CSP is `script-src 'self'` and stays that way.
//
// Mirrors ThemeStore in src/lib/design/theme.svelte.ts: same storage key,
// same fallback to dark, same "system" resolution through matchMedia.
(function () {
  var mode = 'dark';
  try {
    var stored = localStorage.getItem('wc.theme');
    if (stored === 'dark' || stored === 'light' || stored === 'system') {
      mode = stored;
    }
  } catch (_) {
    // Storage unavailable; dark is the documented default.
  }
  var resolved = mode;
  if (mode === 'system') {
    var prefersDark = true;
    try {
      prefersDark = window.matchMedia('(prefers-color-scheme: dark)').matches;
    } catch (_) {
      // No matchMedia; keep the dark default.
    }
    resolved = prefersDark ? 'dark' : 'light';
  }
  document.documentElement.setAttribute('data-theme', resolved);
})();
