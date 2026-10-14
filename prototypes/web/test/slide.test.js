import test from 'node:test';
import assert from 'node:assert/strict';
import { newDeck, normalizeDeck, moveSlide, slideHtml, exportHtml, THEMES } from '../slide/model.js';

test('normalizeDeck rejects junk and clamps hostile values', () => {
  assert.equal(normalizeDeck(null), null);
  assert.equal(normalizeDeck({ slides: [] }), null);
  const d = normalizeDeck({ theme: 'Nope', slides: [{ items: [{ type: 'text', x: 1e9, size: -5, color: 'url(javascript:1)', text: 'hi' }] }] });
  const i = d.slides[0].items[0];
  assert.equal(d.theme, 'Indigo');
  assert.equal(i.x, 2000);
  assert.equal(i.size, 8);
  assert.equal(i.color, '#000000');
});

test('slides reorder', () => {
  const d = newDeck();
  d.slides.push({ items: [] });
  const [a, b, c] = d.slides;
  moveSlide(d, 0, 2);
  assert.deepEqual(d.slides, [b, c, a]);
  moveSlide(d, 0, 99);
  assert.deepEqual(d.slides, [b, c, a]);
});

test('slide HTML escapes text so decks cannot inject markup', () => {
  const html = slideHtml({ items: [{ id: 'x', type: 'text', x: 0, y: 0, w: 10, h: 10, text: '<img src=x onerror=alert(1)>', size: 20, color: '#000', bold: false }] }, 'Indigo');
  assert.ok(!html.includes('<img'));
  assert.ok(exportHtml(newDeck()).includes('<section>'));
  assert.ok(THEMES.Indigo);
});
