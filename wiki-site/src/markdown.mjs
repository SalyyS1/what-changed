export const escapeHtml = (text) => String(text).replace(/[&<>"']/g, (char) => ({
  '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;',
}[char]));
export const slug = (text) => text.toLowerCase().normalize('NFKD').replace(/[\u0300-\u036f]/g, '')
  .replace(/[^a-z0-9]+/g, '-').replace(/^-|-$/g, '');

/** Markdown is deliberately small: raw HTML is always text, never executable. */
export function inline(text, resolveLink = (href) => href) {
  const pattern = /\[([^\]]+)\]\(([^\s)]+)\)|`([^`]+)`|\*\*([^*]+)\*\*/g;
  let result = '', cursor = 0;
  for (const match of text.matchAll(pattern)) {
    result += escapeHtml(text.slice(cursor, match.index));
    if (match[1]) {
      const href = resolveLink(match[2]);
      result += href && (/^https:\/\//.test(href) || /^\/(?!\/)/.test(href) || /^#[a-z0-9-]+$/.test(href))
        ? `<a href="${escapeHtml(href)}">${escapeHtml(match[1])}</a>` : escapeHtml(match[1]);
    } else if (match[3]) result += `<code>${escapeHtml(match[3])}</code>`;
    else result += `<strong>${escapeHtml(match[4])}</strong>`;
    cursor = match.index + match[0].length;
  }
  return result + escapeHtml(text.slice(cursor));
}

export function renderMarkdown(source, resolveLink) {
  const toc = [], output = [], headings = new Map();
  let paragraph = [], list = false, code = null;
  const flush = () => {
    if (paragraph.length) output.push(`<p>${inline(paragraph.join(' '), resolveLink)}</p>`);
    paragraph = [];
    if (list) output.push('</ul>');
    list = false;
  };
  for (const line of source.replace(/\r\n/g, '\n').split('\n')) {
    if (/^```/.test(line)) {
      if (code !== null) { output.push(`<pre><code>${escapeHtml(code.join('\n'))}</code></pre>`); code = null; }
      else { flush(); code = []; }
      continue;
    }
    if (code !== null) { code.push(line); continue; }
    const heading = /^(#{1,4})\s+(.+)$/.exec(line);
    if (heading) {
      flush();
      if (heading[1].length === 1) continue;
      const base = slug(heading[2]), count = headings.get(base) || 0;
      headings.set(base, count + 1);
      const id = count ? `${base}-${count}` : base;
      toc.push({ id, label: heading[2] });
      output.push(`<h${heading[1].length} id="${id}">${inline(heading[2], resolveLink)}</h${heading[1].length}>`);
    } else if (/^- /.test(line)) {
      if (paragraph.length) { output.push(`<p>${inline(paragraph.join(' '), resolveLink)}</p>`); paragraph = []; }
      if (!list) { output.push('<ul>'); list = true; }
      output.push(`<li>${inline(line.slice(2), resolveLink)}</li>`);
    } else if (!line.trim()) flush();
    else { if (list) flush(); paragraph.push(line); }
  }
  flush();
  if (code !== null) throw new Error('Unclosed Markdown code fence');
  return { html: output.join('\n'), toc };
}
