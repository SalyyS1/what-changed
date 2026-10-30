import { attachDemo } from './demo.mjs';
import { attachWorkflow } from './workflow-tour.mjs';

document.querySelectorAll('[data-demo]').forEach(attachDemo);
document.querySelectorAll('[data-workflow]').forEach(attachWorkflow);
const base = document.documentElement.dataset.base;
const legacy = { overview: '', 'first-investigation': 'docs/legacy/', timeline: 'docs/legacy/#timeline', workspace: 'docs/migration-guide/', privacy: 'docs/privacy/', build: 'docs/legacy/#build', status: 'download/' };
if (location.pathname === base && Object.hasOwn(legacy, location.hash.slice(1))) {
  const route = legacy[location.hash.slice(1)];
  if (route) location.replace(`${base}${route}`);
}

const form = document.querySelector('[data-search]');
if (form) {
  const input = form.querySelector('input'), results = document.querySelector('[data-search-results]');
  let index, sequence = 0;
  async function search() {
    const current = ++sequence, query = input.value.trim().toLocaleLowerCase();
    results.replaceChildren();
    if (!query) { results.textContent = 'Search the English documentation. Vietnamese quickstart and glossary are also indexed.'; return; }
    results.textContent = 'Loading local search…';
    try {
      index ||= await fetch(`${base}search-index.json`).then((response) => {
        if (!response.ok) throw new Error('Search unavailable'); return response.json();
      });
      if (current !== sequence) return;
      const terms = query.split(/\s+/).filter(Boolean);
      const matches = index.filter((item) => terms.every((term) => `${item.title} ${item.text}`.toLocaleLowerCase().includes(term)));
      results.replaceChildren();
      if (!matches.length) { results.textContent = 'No matching page. Try “rules”, “stale”, or “privacy”.'; return; }
      const summary = document.createElement('p'); summary.textContent = `${matches.length} matching pages`; results.append(summary);
      const list = document.createElement('ul');
      matches.forEach((item) => {
        const row = document.createElement('li'), link = document.createElement('a'), excerpt = document.createElement('p');
        link.href = `${base}${item.route}`; link.textContent = item.title;
        excerpt.textContent = item.excerpt; row.append(link, excerpt); list.append(row);
      }); results.append(list);
    } catch { if (current === sequence) results.textContent = 'Search is unavailable. Use the complete documentation links below.'; }
  }
  input.addEventListener('input', search); form.addEventListener('submit', (event) => { event.preventDefault(); search(); });
  search();
}

const platform = document.querySelector('[data-platform]');
if (platform) {
  const state = document.querySelector('[data-platform-state]'), download = document.querySelector('[data-download]');
  platform.hidden = false;
  platform.querySelector('select').addEventListener('change', (event) => {
    const supported = event.target.value === 'windows-x64';
    state.textContent = supported ? 'Windows x64 is the first supported platform. Release approval is checked independently.' : 'This platform has no installer. You can read the docs and explore the browser demo.';
    if (download) download.hidden = !supported;
  });
}
