// Document conversions for Folio Write. Pure functions over DOM nodes / strings.

const ALLOWED = new Set(['P', 'H1', 'H2', 'H3', 'BLOCKQUOTE', 'UL', 'OL', 'LI', 'B', 'STRONG', 'I', 'EM', 'U', 'S', 'STRIKE',
  'A', 'BR', 'HR', 'TABLE', 'TBODY', 'THEAD', 'TR', 'TD', 'TH', 'DIV', 'SPAN']);

// Removes scripts, event handlers and unknown elements from imported HTML (keeps their text).
export function sanitize(html) {
  const doc = new DOMParser().parseFromString(`<body>${html}</body>`, 'text/html');
  const walk = (node) => {
    for (const child of [...node.children]) {
      walk(child);
      if (!ALLOWED.has(child.tagName)) {
        if (['SCRIPT', 'STYLE', 'IFRAME', 'OBJECT', 'EMBED'].includes(child.tagName)) child.remove();
        else child.replaceWith(...child.childNodes);
        continue;
      }
      for (const a of [...child.attributes]) {
        const keep = (child.tagName === 'A' && a.name === 'href' && /^(https?:|mailto:|#)/i.test(a.value)) ||
          (a.name === 'style' && /^(text-align:\s*(left|center|right|justify);?\s*)$/i.test(a.value));
        if (!keep) child.removeAttribute(a.name);
      }
      if (child.tagName === 'A') { child.setAttribute('rel', 'noopener'); child.setAttribute('target', '_blank'); }
    }
  };
  walk(doc.body);
  return doc.body.innerHTML;
}

const esc = (s) => s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');

// Tiny Markdown subset -> HTML: headings, quotes, lists, rules, **bold**, *italic*, `code`, [links](url).
export function markdownToHtml(md) {
  const inline = (t) => esc(t)
    .replace(/\*\*(.+?)\*\*/g, '<b>$1</b>').replace(/\*(.+?)\*/g, '<i>$1</i>')
    .replace(/\[(.+?)\]\((https?:[^)\s]+)\)/g, '<a href="$2">$1</a>');
  const out = [];
  let list = null;
  const close = () => { if (list) { out.push(`</${list}>`); list = null; } };
  for (const line of md.split(/\r?\n/)) {
    let m;
    if ((m = /^(#{1,3})\s+(.*)$/.exec(line))) { close(); out.push(`<h${m[1].length}>${inline(m[2])}</h${m[1].length}>`); }
    else if ((m = /^>\s?(.*)$/.exec(line))) { close(); out.push(`<blockquote>${inline(m[1])}</blockquote>`); }
    else if ((m = /^[-*]\s+(.*)$/.exec(line))) { if (list !== 'ul') { close(); out.push('<ul>'); list = 'ul'; } out.push(`<li>${inline(m[1])}</li>`); }
    else if ((m = /^\d+\.\s+(.*)$/.exec(line))) { if (list !== 'ol') { close(); out.push('<ol>'); list = 'ol'; } out.push(`<li>${inline(m[1])}</li>`); }
    else if (/^---+$/.test(line.trim())) { close(); out.push('<hr>'); }
    else if (line.trim() === '') close();
    else { close(); out.push(`<p>${inline(line)}</p>`); }
  }
  close();
  return out.join('\n');
}

// HTML (already sanitized) -> Markdown subset.
export function htmlToMarkdown(html) {
  const root = new DOMParser().parseFromString(`<body>${html}</body>`, 'text/html').body;
  const inline = (n) => [...n.childNodes].map((c) => {
    if (c.nodeType === 3) return c.textContent;
    const t = c.tagName, inner = inline(c);
    if (t === 'B' || t === 'STRONG') return `**${inner}**`;
    if (t === 'I' || t === 'EM') return `*${inner}*`;
    if (t === 'A') return `[${inner}](${c.getAttribute('href') ?? ''})`;
    if (t === 'BR') return '\n';
    return inner;
  }).join('');
  const lines = [];
  for (const n of root.children) {
    const t = n.tagName;
    if (/^H[1-3]$/.test(t)) lines.push(`${'#'.repeat(+t[1])} ${inline(n)}`, '');
    else if (t === 'BLOCKQUOTE') lines.push(`> ${inline(n)}`, '');
    else if (t === 'UL' || t === 'OL') { [...n.children].forEach((li, i) => lines.push(`${t === 'UL' ? '-' : `${i + 1}.`} ${inline(li)}`)); lines.push(''); }
    else if (t === 'HR') lines.push('---', '');
    else lines.push(inline(n), '');
  }
  return lines.join('\n').trim() + '\n';
}

export function htmlToText(html) {
  const root = new DOMParser().parseFromString(`<body>${html}</body>`, 'text/html').body;
  return [...root.children].map((n) => n.innerText ?? n.textContent).join('\n\n');
}

export const countWords = (text) => (text.trim().match(/\S+/g) ?? []).length;
