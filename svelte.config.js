import { vitePreprocess } from '@sveltejs/vite-plugin-svelte';

/**
 * Plain Svelte 5 SPA — deliberately not SvelteKit.
 * No SSR, no adapter, no router build layer: the app is a single local webview
 * whose data comes from typed Tauri commands, so a server framework would add
 * build surface and zero value.
 * @type {import('@sveltejs/vite-plugin-svelte').SvelteConfig}
 */
export default {
  preprocess: vitePreprocess(),
  compilerOptions: {
    // Svelte 5 runes only. Legacy reactive statements are a compile error,
    // which keeps one reactivity model across the whole codebase.
    runes: true,
  },
};
