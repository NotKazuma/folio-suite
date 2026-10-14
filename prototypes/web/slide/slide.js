import { register, run, mount, download, pickFile, store } from '../shared/cmd.js';
import { W, H, THEMES, newDeck, newSlide, newItem, normalizeDeck, slideHtml, exportHtml, moveSlide } from './model.js';

const KEY = 'folio-slide:v1';
let deck;
try { deck = normalizeDeck(JSON.parse(store.get(KEY) ?? 'null')); } catch { deck = null; }
deck ??= newDeck();
let cur = 0, selId = null;

const stage = document.getElementById('stage');
const thumbs = document.getElementById('thumbs');
const props = document.getElementById('props');
const themeSel = document.getElementById('theme');
const presentEl = document.getElementById('present');
themeSel.innerHTML = Object.keys(THEMES).map((t) => `<option>${t}</option>`).join('');

const slide = () => deck.slides[cur];
const item = () => slide().items.find((i) => i.id === selId);

let timer;
function persist() {
  document.getElementById('saved').textContent = 'Saving…';
  clearTimeout(timer);
  timer = setTimeout(() => { store.set(KEY, JSON.stringify(deck)); document.getElementById('saved').textContent = 'Saved in this browser'; }, 400);
}

function fit() {
  const wrap = document.getElementById('stage-wrap');
  const k = Math.min(1, (wrap.clientWidth - 32) / W, (wrap.clientHeight - 32) / H);
  stage.style.transform = `scale(${k})`;
  stage.style.margin = `0 ${-(W * (1 - k))}px ${-(H * (1 - k))}px 0`;
  return k;
}

function renderStage() {
  stage.innerHTML = slideHtml(slide(), deck.theme);
  const root = stage.firstElementChild;
  const el = selId && root.querySelector(`[data-id="${selId}"]`);
  if (el) { el.classList.add('sel'); const h = document.createElement('div'); h.className = 'handle'; h.dataset.handle = '1'; el.style.overflow = 'visible'; el.append(h); }
  props.replaceChildren(...propControls());
  document.getElementById('count').textContent = `Slide ${cur + 1} of ${deck.slides.length}`;
  fit();
}

function renderThumbs() {
  thumbs.replaceChildren(...deck.slides.map((s, i) => {
    const d = document.createElement('div');
    d.className = 'thumb' + (i === cur ? ' sel' : '');
    d.draggable = true;
    d.innerHTML = `<div class="inner">${slideHtml(s, deck.theme)}</div><b>${i + 1}</b>`;
    d.onclick = () => { cur = i; selId = null; render(); };
    d.ondragstart = (e) => e.dataTransfer.setData('text/plain', String(i));
    d.ondragover = (e) => e.preventDefault();
    d.ondrop = (e) => { e.preventDefault(); const from = Number(e.dataTransfer.getData('text/plain')); moveSlide(deck, from, i); cur = i; render(); persist(); };
    return d;
  }));
}

function propControls() {
  const it = item();
  if (!it) return [];
  const mk = (tag, attrs, on) => Object.assign(document.createElement(tag), attrs, { oninput: on });
  const out = [];
  if (it.type === 'text') {
    out.push(mk('input', { type: 'number', value: it.size, min: 8, max: 200, title: 'Font size' }, (e) => { it.size = Number(e.target.value) || 32; refresh(); }));
    out.push(mk('input', { type: 'color', value: /^#[0-9a-f]{6}$/i.test(it.color) ? it.color : '#000000', title: 'Text colour' }, (e) => { it.color = e.target.value; refresh(); }));
    out.push(mk('button', { textContent: 'B', title: 'Bold', className: it.bold ? 'on' : '' }, null));
    out[out.length - 1].onclick = () => { it.bold = !it.bold; refresh(); renderStage(); };
  } else {
    out.push(mk('input', { type: 'color', value: /^#[0-9a-f]{6}$/i.test(it.fill) ? it.fill : '#4f46e5', title: 'Fill colour' }, (e) => { it.fill = e.target.value; refresh(); }));
  }
  return out;
}

function refresh() {
  const el = stage.querySelector(`[data-id="${selId}"]`), it = item();
  if (el && it) {
    el.style.left = it.x + 'px'; el.style.top = it.y + 'px'; el.style.width = it.w + 'px'; el.style.height = it.h + 'px';
    if (it.type === 'text') { el.style.fontSize = it.size + 'px'; el.style.color = it.color; el.style.fontWeight = it.bold ? 700 : 400; }
    else el.style.background = it.fill;
  }
  renderThumbs(); persist();
}

function render() { renderStage(); renderThumbs(); themeSel.value = deck.theme; }

// Pointer interaction: select, drag, resize.
stage.addEventListener('pointerdown', (e) => {
  const k = fit();
  const target = e.target.closest('[data-id]');
  if (!target) { if (selId) { selId = null; renderStage(); } return; }
  if (selId !== target.dataset.id) { selId = target.dataset.id; renderStage(); }
  const it = item(); if (!it || e.target.isContentEditable) return;
  const resizing = e.target.dataset.handle === '1';
  const sx = e.clientX, sy = e.clientY, ox = it.x, oy = it.y, ow = it.w, oh = it.h;
  const move = (m) => {
    const dx = (m.clientX - sx) / k, dy = (m.clientY - sy) / k;
    if (resizing) { it.w = Math.max(20, Math.round(ow + dx)); it.h = Math.max(20, Math.round(oh + dy)); }
    else { it.x = Math.round(ox + dx); it.y = Math.round(oy + dy); }
    refresh();
  };
  const up = () => { removeEventListener('pointermove', move); removeEventListener('pointerup', up); };
  addEventListener('pointermove', move); addEventListener('pointerup', up);
});

stage.addEventListener('dblclick', (e) => {
  const el = e.target.closest('[data-id]'), it = item();
  if (!el || !it || it.type !== 'text') return;
  el.contentEditable = 'plaintext-only';
  el.querySelector('.handle')?.remove();
  el.focus();
  el.onblur = () => { it.text = el.innerText.replace(/\n$/, ''); el.contentEditable = 'false'; renderStage(); renderThumbs(); persist(); };
});

document.addEventListener('keydown', (e) => {
  if (presentEl.classList.contains('on')) {
    if (['ArrowRight', ' ', 'PageDown'].includes(e.key)) { cur = Math.min(cur + 1, deck.slides.length - 1); drawPresent(); }
    else if (['ArrowLeft', 'PageUp'].includes(e.key)) { cur = Math.max(cur - 1, 0); drawPresent(); }
    else if (e.key === 'Escape') endPresent();
    return;
  }
  if (e.target.isContentEditable || /INPUT|SELECT|TEXTAREA/.test(e.target.tagName)) return;
  if ((e.key === 'Delete' || e.key === 'Backspace') && selId) { e.preventDefault(); run('item.del'); }
});

function drawPresent() {
  const k = Math.min(innerWidth / W, innerHeight / H);
  presentEl.innerHTML = `<div style="transform:scale(${k})">${slideHtml(slide(), deck.theme)}</div>`;
}
function endPresent() { presentEl.classList.remove('on'); document.fullscreenElement && document.exitFullscreen?.(); render(); }
presentEl.addEventListener('click', () => { cur = Math.min(cur + 1, deck.slides.length - 1); drawPresent(); });
addEventListener('resize', () => { fit(); if (presentEl.classList.contains('on')) drawPresent(); });

const reg = register;

const add = (layout) => () => { deck.slides.splice(cur + 1, 0, newSlide(layout, THEMES[deck.theme])); cur++; selId = null; render(); persist(); };
reg('slide.add.title', 'New title slide', add('title'));
reg('slide.add.content', 'New content slide', add('content'));
reg('slide.add.blank', 'New blank slide', add('blank'));
reg('slide.dup', 'Duplicate slide', () => {
  const copy = normalizeDeck({ theme: deck.theme, slides: [JSON.parse(JSON.stringify(slide()))] }).slides[0];
  deck.slides.splice(cur + 1, 0, copy); cur++; selId = null; render(); persist();
});
reg('slide.del', 'Delete slide', () => { if (deck.slides.length > 1) { deck.slides.splice(cur, 1); cur = Math.min(cur, deck.slides.length - 1); selId = null; render(); persist(); } });
const addItem = (type) => () => { const it = newItem(type, THEMES[deck.theme]); slide().items.push(it); selId = it.id; render(); persist(); };
reg('item.text', 'Add text box', addItem('text'));
reg('item.rect', 'Add shape', addItem('rect'));
reg('item.del', 'Remove selected item', () => { const s = slide(); s.items = s.items.filter((i) => i.id !== selId); selId = null; render(); persist(); });
reg('present.start', 'Present full screen', () => { selId = null; presentEl.classList.add('on'); presentEl.requestFullscreen?.().catch(() => {}); drawPresent(); }, '');
reg('file.new', 'New presentation', () => { if (confirm('Discard this presentation?')) { deck = newDeck(); cur = 0; selId = null; render(); persist(); } });
reg('file.open', 'Open presentation (.json)', async () => {
  const f = await pickFile('.json,application/json'); if (!f) return;
  let d = null; try { d = normalizeDeck(JSON.parse(f.text)); } catch { /* handled below */ }
  if (!d) { alert('That file is not a Folio Slide presentation.'); return; }
  deck = d; cur = 0; selId = null; render(); persist();
});
reg('file.save.json', 'Save presentation (.json)', () => download('presentation.json', JSON.stringify(deck, null, 2), 'application/json'), 's');
reg('file.save.html', 'Export as standalone HTML', () => download('presentation.html', exportHtml(deck), 'text/html'));

themeSel.onchange = () => { deck.theme = themeSel.value; render(); persist(); };
mount();
render();
