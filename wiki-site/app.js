const pages = [
  { id: 'overview', label: 'Overview', meta: 'The product', title: 'A calm record of change', body: 'What Changed watches local signals and presents the surrounding evidence when a project, service, port, or update stops behaving as expected.', bullets: ['No account, cloud sync, telemetry, or runtime network client.', 'Local SQLite history with explicit privacy and retention controls.', 'Evidence is ranked as correlation; the app does not claim a cause.'], links: [['Product scope', '../PRODUCT.md'], ['Architecture', '../ARCHITECTURE.md']] },
  { id: 'first-investigation', label: 'First investigation', meta: 'Core workflow', title: 'From symptom to evidence', body: 'Describe what stopped working, choose a time window, review ranked suspects, and open the underlying event details.', bullets: ['Use a known-good checkpoint when you have one.', 'Sensitive values are shown as redacted or value-free changes.', 'The AI Troubleshooting Pack copies a redacted context only after confirmation.'], links: [['First investigation guide', '../docs/first-investigation.md'], ['Privacy rules', '../PRIVACY.md']] },
  { id: 'timeline', label: 'Timeline & search', meta: 'Explore history', title: 'Find the moment things changed', body: 'The timeline is paginated and searchable. Filters and event details keep the record useful without dumping raw machine state into the interface.', bullets: ['Search by plain words, entity names, ports, versions, or services.', 'Open a details drawer for before/after evidence and relationships.', 'Watched folders stay inside the folder boundary you choose.'], links: [['Timeline guide', '../docs/timeline-and-search.md'], ['Collector boundary', '../COLLECTORS.md']] },
  { id: 'workspace', label: 'Workspaces & checkpoints', meta: 'Project context', title: 'Give a project a known-good starting point', body: 'Create a workspace from a project root, mark it working, then investigate changes relative to that saved baseline.', bullets: ['The workspace stores a boundary and branch identity, not secret values.', 'Mark working creates a checkpoint owned by that project.', 'Maps show relationships, with a simpler 2D fallback when needed.'], links: [['Checkpoints and maps', '../docs/checkpoints-and-maps.md'], ['Settings and privacy', '../docs/settings-and-privacy.md']] },
  { id: 'privacy', label: 'Privacy & security', meta: 'Trust boundary', title: 'Useful evidence without keeping secrets', body: 'Recognized secret-bearing values are redacted before persistence or comparison. Paths and non-secret evidence remain local because they are needed to explain a change.', bullets: ['No administrator requirement in the supported collector boundary.', 'Uninstall keeps history unless the user explicitly opts in to deletion.', 'The updater is opt-in and has no production feed or release key yet.'], links: [['Privacy', '../PRIVACY.md'], ['Security', '../SECURITY.md'], ['Known limitations', '../docs/known-limitations.md']] },
  { id: 'build', label: 'Build & contribute', meta: 'For maintainers', title: 'Run it locally', body: 'The public repository is a source preview. Build it on Windows with the documented MSVC Rust toolchain, Bun, and WebView2 runtime.', bullets: ['bun install', 'bun run tauri dev', 'bun run verify', 'bun run tauri build'], links: [['Contributing', '../CONTRIBUTING.md'], ['Release audit', '../docs/release-audit.md'], ['Acceptance walkthrough', '../docs/acceptance-walkthrough.md']] },
  { id: 'status', label: 'Status & limitations', meta: 'Release boundary', title: 'Source preview, not a release', body: 'The source and local QA candidate are available for review. Publication remains gated by installed acceptance, upgrade and uninstall evidence, accessibility, resource and soak checks, clean-tree verification, and explicit approval.', bullets: ['The installer is unsigned and may trigger an Unknown publisher warning.', 'Docker-live coverage and several Windows data sources remain environment-bound.', 'The temporary web wiki is the readable route until a later hosted documentation pass.'], links: [['Roadmap', '../ROADMAP.md'], ['Go/no-go note', '../docs/public-v1-go-no-go.md'], ['Third-party notices', '../THIRD_PARTY_NOTICES.md']] },
];

const nav = document.querySelector('#nav');
const pagesRoot = document.querySelector('#pages');
const search = document.querySelector('#search');
const sourceBase = 'https://github.com/SalyyS1/what-changed/blob/main/';

function sourceLink(href) {
  return href.startsWith('../') ? `${sourceBase}${href.slice(3)}` : href;
}

function render() {
  const query = search.value.trim().toLowerCase();
  const visible = pages.filter((page) => `${page.label} ${page.title} ${page.body} ${page.bullets.join(' ')}`.toLowerCase().includes(query));
  nav.innerHTML = pages.map((page) => `<a href="#${page.id}" data-id="${page.id}">${page.label}</a>`).join('');
  pagesRoot.innerHTML = visible.length ? visible.map((page) => `
    <section class="page" id="${page.id}">
      <p class="page-meta">${page.meta}</p>
      <h3>${page.title}</h3>
      <p>${page.body}</p>
      <ul>${page.bullets.map((item) => `<li>${item}</li>`).join('')}</ul>
      <div class="page-links">${page.links.map(([label, href]) => `<a href="${sourceLink(href)}">${label} ↗</a>`).join('')}</div>
    </section>`).join('') : '<p class="empty">No matching page. Try a broader search.</p>';
  const current = location.hash.slice(1) || 'overview';
  nav.querySelectorAll('a').forEach((link) => link.setAttribute('aria-current', link.dataset.id === current ? 'true' : 'false'));
}

search.addEventListener('input', render);
window.addEventListener('hashchange', render);
render();
