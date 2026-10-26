/**
 * The token palette as data, mirrored from `src/styles/tokens.css`.
 *
 * CSS is the runtime source of truth; this module exists so the contrast test
 * can assert every text/surface pair for both themes without a DOM, and so
 * the Three.js palette can read colours without `getComputedStyle`.
 * A drift test compares these values against the stylesheet.
 */

export type ThemeName = 'dark' | 'light';

export interface Palette {
  background: string;
  'surface-1': string;
  'surface-2': string;
  'surface-3': string;
  'border-subtle': string;
  'border-active': string;
  'text-primary': string;
  'text-secondary': string;
  'text-muted': string;
  accent: string;
  'accent-contrast': string;
  success: string;
  warning: string;
  danger: string;
  info: string;
}

export const palettes: Record<ThemeName, Palette> = {
  dark: {
    background: '#0b0c0e',
    'surface-1': '#121417',
    'surface-2': '#171a1f',
    'surface-3': '#1e2228',
    'border-subtle': '#22262d',
    'border-active': '#323945',
    'text-primary': '#e8eaed',
    'text-secondary': '#a5abb6',
    'text-muted': '#828b9a',
    accent: '#5b8cff',
    'accent-contrast': '#0b0c0e',
    success: '#37b37e',
    warning: '#d9a32b',
    danger: '#ea5b5f',
    info: '#4ba3c7',
  },
  light: {
    background: '#f6f7f9',
    'surface-1': '#ffffff',
    'surface-2': '#f1f3f6',
    'surface-3': '#e7eaef',
    'border-subtle': '#dfe3e9',
    'border-active': '#b9c1cd',
    'text-primary': '#17191d',
    'text-secondary': '#4b5261',
    'text-muted': '#606775',
    accent: '#2d5fdc',
    'accent-contrast': '#ffffff',
    success: '#1a754c',
    warning: '#866110',
    danger: '#bc383c',
    info: '#1c6f8d',
  },
};

export type SurfaceToken = 'background' | 'surface-1' | 'surface-2' | 'surface-3';
export type TextToken = 'text-primary' | 'text-secondary' | 'text-muted';
export type StatusToken = 'accent' | 'success' | 'warning' | 'danger' | 'info';

export const surfaceTokens: readonly SurfaceToken[] = [
  'background',
  'surface-1',
  'surface-2',
  'surface-3',
];
export const textTokens: readonly TextToken[] = [
  'text-primary',
  'text-secondary',
  'text-muted',
];
export const statusTokens: readonly StatusToken[] = [
  'accent',
  'success',
  'warning',
  'danger',
  'info',
];

/**
 * Every (foreground, background) pair a component is allowed to render text
 * with, and the WCAG level it must meet. All three text tiers and every
 * status colour are used at body size (11–13.5px), so each is held to AA
 * normal, 4.5:1, on every surface it can sit on. The 3:1 large-text level
 * is reserved for a pair that is provably only ever rendered >= 18.66px
 * bold; none exists yet.
 */
export interface TextPair {
  fg: TextToken | StatusToken | 'accent-contrast';
  bg: SurfaceToken | 'accent';
  minRatio: 4.5 | 3;
}

export const textPairs: readonly TextPair[] = [
  ...surfaceTokens.flatMap<TextPair>((bg) => [
    { fg: 'text-primary', bg, minRatio: 4.5 },
    { fg: 'text-secondary', bg, minRatio: 4.5 },
    { fg: 'text-muted', bg, minRatio: 4.5 },
    ...statusTokens.map<TextPair>((fg) => ({ fg, bg, minRatio: 4.5 })),
  ]),
  { fg: 'accent-contrast', bg: 'accent', minRatio: 4.5 },
];
