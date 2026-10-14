import { register, run, mount, download, pickFile, store } from '../shared/cmd.js';
import { sanitize, markdownToHtml, htmlToMarkdown, htmlToText, countWords } from './convert.js';

const doc = document.getElementById('doc');
const KEY = 'folio-write:doc';
const words = document.getElementById('words');
const savedEl = document.getElementById('saved');

const exec = (cmd, val) => { doc.focus(); document.execCommand(cmd, false, val); update(); };
const setHtml = (html) => { doc.innerHTML = sanitize(html) || '<p><br></p>'; update(); };

doc.innerHTML = sanitize(store.get(KEY) ?? '') ||
  '<h1>Untitled document</h1><p>Start writing. Press Ctrl+K to search every command.</p>';

let timer;
function update() {
  words.textContent = `${countWords(doc.innerText)} words`;
  clearTimeout(timer);
  savedEl.textContent = 'Saving…';
  timer = setTimeout(() => { store.set(KEY, doc.innerHTML); savedEl.textContent = 'Saved in this browser'; }, 400);
}
doc.addEventListener('input', update);

for (const [id, title, cmd, key] of [
  ['fmt.bold', 'Bold', 'bold', 'b'], ['fmt.italic', 'Italic', 'italic', 'i'], ['fmt.underline', 'Underline', 'underline', 'u'],
  ['fmt.strike', 'Strikethrough', 'strikeThrough', ''], ['list.bullets', 'Bulleted list', 'insertUnorderedList', ''],
  ['list.numbers', 'Numbered list', 'insertOrderedList', ''], ['align.left', 'Align left', 'justifyLeft', ''],
  ['align.center', 'Align center', 'justifyCenter', ''], ['align.right', 'Align right', 'justifyRight', ''],
  ['insert.rule', 'Insert horizontal rule', 'insertHorizontalRule', ''], ['edit.undo', 'Undo', 'undo', 'z'], ['edit.redo', 'Redo', 'redo', 'y'],
]) register(id, title, () => exec(cmd), key);

register('insert.link', 'Insert link', () => {
  const url = prompt('Link address (https://…)');
  if (url && /^(https?:|mailto:)/i.test(url)) exec('createLink', url);
});
register('insert.table', 'Insert table', () => {
  const cols = Number(prompt('Columns (1-12)', '3')) || 0;
  const rows = Number(prompt('Rows (1-100)', '3')) || 0;
  if (cols < 1 || cols > 12 || rows < 1 || rows > 100) return;
  const row = `<tr>${'<td><br></td>'.repeat(cols)}</tr>`;
  exec('insertHTML', `<table><tbody>${row.repeat(rows)}</tbody></table><p><br></p>`);
});
register('file.new', 'New document', () => { if (confirm('Discard the current document?')) setHtml('<h1>Untitled document</h1><p><br></p>'); });
register('file.open', 'Open file (.html, .md, .txt)', async () => {
  const f = await pickFile('.html,.htm,.md,.markdown,.txt');
  if (!f) return;
  if (/\.html?$/i.test(f.name)) setHtml(f.text);
  else if (/\.(md|markdown)$/i.test(f.name)) setHtml(markdownToHtml(f.text));
  else setHtml(f.text.split(/\n{2,}/).map((p) => `<p>${p.replace(/&/g, '&amp;').replace(/</g, '&lt;')}</p>`).join(''));
});
register('file.print', 'Print or save as PDF', () => window.print(), 'p');
register('file.save.html', 'Save as HTML', () =>
  download('document.html', `<!doctype html><meta charset="utf-8"><title>Document</title><body>${sanitize(doc.innerHTML)}</body>`, 'text/html'), 's');
register('file.save.md', 'Save as Markdown', () => download('document.md', htmlToMarkdown(sanitize(doc.innerHTML))));
register('file.save.txt', 'Save as plain text', () => download('document.txt', htmlToText(sanitize(doc.innerHTML))));
register('edit.find', 'Find and replace', () => {
  const q = prompt('Find');
  if (!q) return;
  const r = prompt('Replace with (leave empty to just count)');
  const re = new RegExp(q.replace(/[.*+?^${}()|[\]\\]/g, '\\$&'), 'gi');
  const n = (doc.innerText.match(re) ?? []).length;
  if (r === null || r === '') { alert(`${n} match${n === 1 ? '' : 'es'}`); return; }
  // Replace only inside text nodes so tags are never touched.
  const w = document.createTreeWalker(doc, NodeFilter.SHOW_TEXT);
  for (let node = w.nextNode(); node; node = w.nextNode()) node.textContent = node.textContent.replace(re, r);
  update();
});

document.getElementById('block').onchange = (e) => exec('formatBlock', e.target.value === 'p' ? 'p' : e.target.value);
document.getElementById('exp').onchange = (e) => { if (e.target.value) run(`file.save.${e.target.value}`); e.target.value = ''; };
mount();
update();
