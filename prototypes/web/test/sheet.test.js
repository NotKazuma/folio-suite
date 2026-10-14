import test from 'node:test';
import assert from 'node:assert/strict';
import { Sheet, colName, colIndex, toCsv, fromCsv } from '../sheet/engine.js';

test('column names round-trip', () => {
  assert.equal(colName(0), 'A');
  assert.equal(colName(25), 'Z');
  assert.equal(colName(26), 'AA');
  assert.equal(colIndex('AA'), 26);
});

test('arithmetic honours precedence and parentheses', () => {
  const s = new Sheet({ A1: '=1+2*3', A2: '=(1+2)*3', A3: '=2^3^2', A4: '=-2+5', A5: '=50%' });
  assert.deepEqual(['A1', 'A2', 'A3', 'A4', 'A5'].map((a) => s.value(a)), [7, 9, 64, 3, 0.5]);
});

test('references, ranges and functions', () => {
  const s = new Sheet({ A1: '10', A2: '20', A3: '30', B1: '=SUM(A1:A3)', B2: '=AVERAGE(A1:A3)', B3: '=MAX(A1:A3)-MIN(A1:A3)', B4: '=IF(B1>50,"big","small")' });
  assert.equal(s.value('B1'), 60);
  assert.equal(s.value('B2'), 20);
  assert.equal(s.value('B3'), 20);
  assert.equal(s.value('B4'), 'big');
});

test('editing a cell recalculates dependants', () => {
  const s = new Sheet({ A1: '1', B1: '=A1*2' });
  assert.equal(s.value('B1'), 2);
  s.set('A1', '5');
  assert.equal(s.value('B1'), 10);
});

test('errors are values, never crashes', () => {
  const s = new Sheet({ A1: '=1/0', A2: '=A1+1', A3: '=NOPE(1)', B1: '=B2', B2: '=B1', C1: '=1+', D1: '=SUM(' });
  assert.equal(s.value('A1'), '#DIV/0!');
  assert.equal(s.value('A2'), '#DIV/0!');
  assert.equal(s.value('A3'), '#NAME?');
  assert.equal(s.value('B1'), '#CYCLE!');
  assert.equal(s.value('C1'), '#VALUE!');
  assert.equal(s.value('D1'), '#VALUE!');
});

test('text, concatenation and booleans', () => {
  const s = new Sheet({ A1: 'hello', B1: '=A1&" "&"world"', C1: '=UPPER(A1)', D1: '=1<2' });
  assert.equal(s.value('B1'), 'hello world');
  assert.equal(s.value('C1'), 'HELLO');
  assert.equal(s.value('D1'), true);
});

test('CSV round-trips quotes and commas', () => {
  const cells = fromCsv('name,note\n"Smith, J","say ""hi"""\n');
  assert.equal(cells.A2, 'Smith, J');
  assert.equal(cells.B2, 'say "hi"');
  assert.equal(toCsv(new Sheet(cells), 2, 2), 'name,note\n"Smith, J","say ""hi"""\n');
});
