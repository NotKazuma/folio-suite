// Folio Slide deck model: plain data plus pure helpers (no DOM), so it can be tested under Node.
export const W = 960, H = 540;
export const THEMES = {
  Indigo: { bg: '#ffffff', ink: '#1b1b2f', accent: '#4f46e5' },
  Night: { bg: '#14141c', ink: '#f2f2f6', accent: '#22d3ee' },
  Sunrise: { bg: '#fff7ed', ink: '#3b1d0b', accent: '#ea580c' },
  Forest: { bg: '#f0fdf4', ink: '#10301c', accent: '#16a34a' },
};

let nextId = 1;
export const newId = () => `i${Date.now().toString(36)}${nextId++}`;

export function newItem(type, theme) {
  const base = { id: newId(), type, x: 120, y: 120, w: 360, h: 120 };
  if (type === 'rect') return { ...base, h: 160, fill: theme.accent };
  return { ...base, text: 'Text', size: 36, color: theme.ink, bold: false };
}

export function newSlide(layout, theme) {
  const t = (text, x, y, w, h, size, bold) => ({ ...newItem('text', theme), text, x, y, w, h, size, bold });
  if (layout === 'title') return { items: [t('Presentation title', 80, 180, 800, 100, 60, true), t('Subtitle', 80, 300, 800, 60, 28, false)] };
  if (layout === 'content') return { items: [t('Slide title', 60, 40, 840, 80, 44, true), t('• First point\n• Second point\n• Third point', 60, 150, 840, 320, 30, false)] };
  return { items: [] };
}

export function newDeck() {
  const theme = 'Indigo';
  return { theme, slides: [newSlide('title', THEMES[theme]), newSlide('content', THEMES[theme])] };
}

// Accepts untrusted JSON (opened files, old storage) and returns a safe deck or null.
export function normalizeDeck(d) {
  if (!d || typeof d !== 'object' || !Array.isArray(d.slides) || d.slides.length === 0 || d.slides.length > 500) return null;
  const num = (v, lo, hi, dflt) => (typeof v === 'number' && Number.isFinite(v) ? Math.min(hi, Math.max(lo, v)) : dflt);
  const color = (v, dflt) => (typeof v === 'string' && /^#[0-9a-f]{3,8}$/i.test(v) ? v : dflt);
  const theme = THEMES[d.theme] ? d.theme : 'Indigo';
  return {
    theme,
    slides: d.slides.map((s) => ({
      items: (Array.isArray(s?.items) ? s.items : []).slice(0, 200).map((i) => ({
        id: newId(), type: i?.type === 'rect' ? 'rect' : 'text',
        x: num(i?.x, -2000, 2000, 100), y: num(i?.y, -2000, 2000, 100), w: num(i?.w, 10, 2000, 200), h: num(i?.h, 10, 2000, 100),
        text: typeof i?.text === 'string' ? i.text.slice(0, 5000) : '', size: num(i?.size, 8, 200, 32),
        color: color(i?.color, '#000000'), fill: color(i?.fill, '#4f46e5'), bold: i?.bold === true,
      })),
    })),
  };
}

export function moveSlide(deck, from, to) {
  if (from === to || from < 0 || to < 0 || from >= deck.slides.length || to >= deck.slides.length) return;
  deck.slides.splice(to, 0, deck.slides.splice(from, 1)[0]);
}

const esc = (s) => String(s).replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');

// One slide as static HTML (used by the editor stage, present mode and the HTML export).
export function slideHtml(slide, theme) {
  const t = THEMES[theme] ?? THEMES.Indigo;
  const items = slide.items.map((i) => i.type === 'rect'
    ? `<div data-id="${i.id}" style="position:absolute;left:${i.x}px;top:${i.y}px;width:${i.w}px;height:${i.h}px;background:${i.fill};border-radius:8px"></div>`
    : `<div data-id="${i.id}" style="position:absolute;left:${i.x}px;top:${i.y}px;width:${i.w}px;height:${i.h}px;font:${i.bold ? 700 : 400} ${i.size}px/1.25 system-ui,sans-serif;color:${i.color};white-space:pre-wrap;overflow:hidden">${esc(i.text)}</div>`).join('');
  return `<div style="position:relative;width:${W}px;height:${H}px;background:${t.bg};overflow:hidden">${items}</div>`;
}

export function exportHtml(deck) {
  const slides = deck.slides.map((s) => `<section>${slideHtml(s, deck.theme)}</section>`).join('');
  return `<!doctype html><meta charset="utf-8"><title>Presentation</title><meta name="viewport" content="width=device-width,initial-scale=1">
<style>body{margin:0;background:#000;display:grid;place-items:center;height:100vh;overflow:hidden}section{display:none;transform-origin:center}section.on{display:block}</style>
${slides}<script>
var s=[].slice.call(document.querySelectorAll('section')),i=0;
function show(){s.forEach(function(e,k){e.className=k===i?'on':'';e.style.transform='scale('+Math.min(innerWidth/${W},innerHeight/${H})+')'})}
addEventListener('keydown',function(e){if(e.key==='ArrowRight'||e.key===' ')i=Math.min(i+1,s.length-1);else if(e.key==='ArrowLeft')i=Math.max(i-1,0);show()});
addEventListener('click',function(){i=Math.min(i+1,s.length-1);show()});addEventListener('resize',show);show();
</script>`;
}
