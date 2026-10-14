import { register, mount, download, pickFile, store } from '../shared/cmd.js';
import { Sheet, colName, addr, toCsv, fromCsv } from './engine.js';

const COLS = 26, ROWS = 100, KEY = 'folio-sheet:v1';
const grid = document.getElementById('grid');
const formula = document.getElementById('formula');
const refEl = document.getElementById('ref');
const sumEl = document.getElementById('sum');
const savedEl = document.getElementById('saved');

let saved = {};
try { saved = JSON.parse(store.get(KEY) ?? '{}') || {}; } catch { saved = {}; }
const sheet = new Sheet(saved.cells && typeof saved.cells === 'object' ? saved.cells : { A1: 'Item', B1: 'Cost', A2: 'Coffee', B2: '4.5', A3: 'Lunch', B3: '12', A4: 'Total', B4: '=SUM(B2:B3)' });
const bold = new Set(Array.isArray(saved.bold) ? saved.bold : ['A1', 'B1']);
let cur = { c: 0, r: 0 };
let editing = null;

const tds = [];
function build() {
  const head = '<tr><th class="rh"></th>' + Array.from({ length: COLS }, (_, c) => `<th>${colName(c)}</th>`).join('') + '</tr>';
  const rows = [];
  for (let r = 0; r < ROWS; r++) rows.push(`<tr><th class="rh">${r + 1}</th>${'<td></td>'.repeat(COLS)}</tr>`);
  grid.innerHTML = head + rows.join('');
  const trs = [...grid.rows].slice(1);
  trs.forEach((tr, r) => { tds[r] = [...tr.cells].slice(1); });
}

function paint(c, r) {
  const a = addr(c, r), td = tds[r][c], v = sheet.value(a);
  td.className = '';
  if (typeof v === 'number') { td.classList.add('num'); td.textContent = Number.isInteger(v) ? String(v) : String(+v.toFixed(10)); }
  else if (typeof v === 'string' && /^#[A-Z/0!?]+$/.test(v) && sheet.raw(a)[0] === '=') { td.classList.add('err'); td.textContent = v; }
  else td.textContent = typeof v === 'boolean' ? String(v).toUpperCase() : v;
  if (bold.has(a)) td.classList.add('b');
  if (c === cur.c && r === cur.r) td.classList.add('sel');
}
const paintAll = () => { for (let r = 0; r < ROWS; r++) for (let c = 0; c < COLS; c++) paint(c, r); };

let timer;
function persist() {
  savedEl.textContent = 'Saving…';
  clearTimeout(timer);
  timer = setTimeout(() => { store.set(KEY, JSON.stringify({ cells: sheet.cells, bold: [...bold] })); savedEl.textContent = 'Saved in this browser'; }, 400);
}

function select(c, r) {
  c = Math.max(0, Math.min(COLS - 1, c)); r = Math.max(0, Math.min(ROWS - 1, r));
  const old = cur; cur = { c, r };
  paint(old.c, old.r); paint(c, r);
  tds[r][c].scrollIntoView({ block: 'nearest', inline: 'nearest' });
  const a = addr(c, r);
  refEl.value = a; formula.value = sheet.raw(a);
  const v = sheet.value(a);
  sumEl.textContent = typeof v === 'number' ? `Value: ${v}` : '—';
}

function commit(a, raw) { sheet.set(a, raw); paintAll(); persist(); }

function startEdit(initial) {
  const td = tds[cur.r][cur.c], a = addr(cur.c, cur.r);
  const input = document.createElement('input');
  input.value = initial ?? sheet.raw(a);
  td.textContent = ''; td.append(input); input.focus();
  editing = { a, input };
  input.onkeydown = (e) => {
    if (e.key === 'Enter') { e.preventDefault(); finish(true); select(cur.c, cur.r + 1); }
    else if (e.key === 'Tab') { e.preventDefault(); finish(true); select(cur.c + 1, cur.r); }
    else if (e.key === 'Escape') { finish(false); }
    e.stopPropagation();
  };
  input.onblur = () => finish(true);
}
function finish(save) {
  if (!editing) return;
  const { a, input } = editing; editing = null;
  input.onblur = null;
  if (save) commit(a, input.value); else paintAll();
  select(cur.c, cur.r); grid.parentElement.focus?.();
}

grid.addEventListener('mousedown', (e) => {
  const td = e.target.closest('td'); if (!td || editing) return;
  select(td.cellIndex - 1, td.parentElement.rowIndex - 1);
});
grid.addEventListener('dblclick', (e) => { if (e.target.closest('td')) startEdit(); });
formula.addEventListener('keydown', (e) => { if (e.key === 'Enter') { commit(addr(cur.c, cur.r), formula.value); select(cur.c, cur.r + 1); } });

document.addEventListener('keydown', (e) => {
  if (editing || e.target === formula || e.ctrlKey || e.metaKey) return;
  const move = { ArrowUp: [0, -1], ArrowDown: [0, 1], ArrowLeft: [-1, 0], ArrowRight: [1, 0] }[e.key];
  if (move) { e.preventDefault(); select(cur.c + move[0], cur.r + move[1]); }
  else if (e.key === 'Enter' || e.key === 'F2') { e.preventDefault(); startEdit(); }
  else if (e.key === 'Delete' || e.key === 'Backspace') commit(addr(cur.c, cur.r), '');
  else if (e.key.length === 1 && !e.altKey) { e.preventDefault(); startEdit(e.key); }
});

const extent = () => {
  let mc = 0, mr = 0;
  for (const a of Object.keys(sheet.cells)) {
    const m = /^([A-Z]+)(\d+)$/.exec(a); if (!m) continue;
    mr = Math.max(mr, +m[2]);
    mc = Math.max(mc, [...m[1]].reduce((n, ch) => n * 26 + ch.charCodeAt(0) - 64, 0));
  }
  return [mc, mr];
};

register('file.new', 'New spreadsheet', () => { if (confirm('Discard this spreadsheet?')) { sheet.cells = {}; sheet.cache.clear(); bold.clear(); paintAll(); select(0, 0); persist(); } });
register('file.open', 'Open CSV file', async () => {
  const f = await pickFile('.csv,.tsv,.txt'); if (!f) return;
  sheet.cells = fromCsv(f.text); sheet.cache.clear(); bold.clear(); paintAll(); select(0, 0); persist();
});
register('file.save.csv', 'Save as CSV', () => { const [c, r] = extent(); download('sheet.csv', toCsv(sheet, c, r), 'text/csv'); }, 's');
register('fmt.bold', 'Bold cell', () => { const a = addr(cur.c, cur.r); bold.has(a) ? bold.delete(a) : bold.add(a); paint(cur.c, cur.r); persist(); }, 'b');
register('edit.clear', 'Clear cell', () => commit(addr(cur.c, cur.r), ''));
function sortColumn(dir) {
  const [, rows] = extent(), col = cur.c;
  const data = [];
  for (let r = 0; r < rows; r++) data.push(Array.from({ length: COLS }, (_, c) => sheet.raw(addr(c, r))));
  const key = (row) => { const v = row[col]; return v !== '' && !Number.isNaN(Number(v)) ? Number(v) : v.toLowerCase(); };
  const empty = (row) => row[col] === '';
  data.sort((a, b) => empty(a) - empty(b) || (key(a) < key(b) ? -dir : key(a) > key(b) ? dir : 0));
  sheet.cells = {};
  data.forEach((row, r) => row.forEach((v, c) => { if (v !== '') sheet.cells[addr(c, r)] = v; }));
  sheet.cache.clear(); paintAll(); persist();
}
register('sort.asc', 'Sort column A to Z', () => sortColumn(1));
register('sort.desc', 'Sort column Z to A', () => sortColumn(-1));

build(); paintAll(); select(0, 0); mount();
