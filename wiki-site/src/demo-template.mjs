import { booking } from './demo-data.mjs';
import { escapeHtml as e } from './markdown.mjs';

export function demoTemplate(compact = false, options = {}) {
  const landing = Boolean(options.landing);
  const auto3d = Boolean(options.auto3d);
  const threePanel = `<div class="three-panel" data-three-panel hidden><div class="demo-controls"><button data-reset type="button">Reset view</button><button data-fit type="button">Fit selected</button><label>Zoom <input type="range" min="0.7" max="1.8" step="0.1" value="1" data-zoom></label><label><input type="checkbox" data-explode checked>Separate layers</label><label>Visible layer <select data-layer><option value="all">All layers</option><option value="0">Owner intent</option><option value="1">Scenarios</option><option value="2">Results</option></select></div><div class="three-canvas" data-three-canvas role="img" aria-label="Three-layer booking evidence model. Use the evidence cards above for keyboard access."></div><p class="small">The model is the same six nodes as the cards. Lines mean requires, verifies, or observed_by; distance carries no verdict.</p><button data-close-3d type="button">Close 3D and release resources</button></div>`;
  return `<section class="workbench${landing ? ' workbench--landing' : ''}" data-demo${auto3d ? ' data-auto-3d="true"' : ''} aria-label="Synthetic booking evidence workbench">
    <div class="workbench-bar"><span class="eyebrow">${booking.label}</span><span class="small">No live testing</span></div>
    <div class="demo-controls" data-enhanced hidden>
      <label>Evidence state <select data-state>${Object.entries(booking.states).map(([key, value]) => `<option value="${key}">${e(value.label)}</option>`).join('')}</select></label>
      <label>Compare <select data-build><option value="after">Candidate · ${booking.after}</option><option value="before">Before · ${booking.before}</option></select></label>
      ${compact ? '' : '<button type="button" data-load-3d>Load 3D workbench</button>'}
    </div>
    <div class="demo-summary" aria-live="polite"><strong data-verdict>Passed · ${booking.after}</strong><span data-explanation>${e(booking.states.current.explanation)}</span></div>
    ${landing ? threePanel : ''}
    <div class="evidence-layers">${['Owner intent', 'Test scenarios', 'Observed result'].map((label, layer) => `<section class="evidence-layer"><h3><span class="step-number">0${layer + 1}</span>${label}</h3>${booking.nodes.filter((node) => node.layer === layer).map((node) => `<details class="evidence-node" data-node="${node.id}"><summary><span class="small">${node.kind}</span><strong>${node.label}</strong></summary><p>${node.detail}</p><button type="button" data-select="${node.id}" data-enhanced hidden>Inspect connections</button></details>`).join('')}</section>`).join('')}</div>
    <details class="connection-list"><summary>Read all explicit connections</summary><ul>${booking.edges.map((edge) => `<li>${booking.nodes.find((node) => node.id === edge.from).label} <code>${edge.type}</code> ${booking.nodes.find((node) => node.id === edge.to).label}</li>`).join('')}</ul></details>
    <div class="inspector" data-inspector aria-live="polite">Open a card to read its evidence. The list and 3D view describe these same six nodes.</div>
    ${landing ? '' : threePanel}
    <noscript><p>The complete 2D cards and connection list work without JavaScript. Interactive state comparison and optional 3D need JavaScript.</p></noscript>
  </section>`;
}
