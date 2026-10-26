/** Canonical onboarding copy. Kept in one module so UI and tests cannot drift. */
export const onboardingCopy = {
  introTitle: 'It worked before. What changed?',
  introBody: 'What Changed keeps a local, plain-language history of the changes around your machine.',
  promise: 'Your history stays on this PC.',
  privacy: 'We record that a value changed, never the value itself.',
  foldersTitle: 'Watch your project folders',
  foldersBody: 'Configuration changes are often the highest-value signal when a project stops working.',
  emptyHome: 'We just started watching. Come back after something changes.',
} as const;
