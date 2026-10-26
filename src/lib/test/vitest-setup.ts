/**
 * Registers testing-library's `afterEach(cleanup)` so each `render()` starts
 * from an empty document. Vitest runs with `globals: false`, so the library
 * cannot hook itself; without this, a second render in one file finds the
 * first one still mounted and every `getBy*` reports duplicates.
 */
import '@testing-library/svelte/vitest';
