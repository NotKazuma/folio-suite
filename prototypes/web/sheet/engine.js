// Folio Sheet calculation engine: a formula parser and a lazy, cycle-safe evaluator.
// Pure module (no DOM) so it can be tested under Node.

export const colName = (i) => { let s = ''; for (i += 1; i > 0; i = Math.floor((i - 1) / 26)) s = String.fromCharCode(65 + ((i - 1) % 26)) + s; return s; };
export const colIndex = (s) => [...s].reduce((n, c) => n * 26 + c.charCodeAt(0) - 64, 0) - 1;
export const addr = (c, r) => `${colName(c)}${r + 1}`;

export class FormulaError extends Error {}
const err = (code) => { throw new FormulaError(code); };

const TOKEN = /\s*(?:(\d+\.?\d*(?:[eE][+-]?\d+)?)|("(?:[^"]|"")*")|([A-Za-z_][A-Za-z0-9_.]*)\(|(\$?[A-Za-z]{1,3}\$?\d+(?::\$?[A-Za-z]{1,3}\$?\d+)?)|(TRUE|FALSE)\b|(<=|>=|<>|[-+*/^&=<>%(),:]))/y;

export function tokenize(src) {
  const out = [];
  let pos = 0;
  while (pos < src.length) {
    TOKEN.lastIndex = pos;
    const m = TOKEN.exec(src);
    if (!m) { if (/^\s*$/.test(src.slice(pos))) break; err('#NAME?'); }
    pos = TOKEN.lastIndex;
    if (m[1] !== undefined) out.push({ t: 'num', v: parseFloat(m[1]) });
    else if (m[2] !== undefined) out.push({ t: 'str', v: m[2].slice(1, -1).replace(/""/g, '"') });
    else if (m[3] !== undefined) out.push({ t: 'fn', v: m[3].toUpperCase() });
    else if (m[4] !== undefined) out.push({ t: 'ref', v: m[4].replace(/\$/g, '').toUpperCase() });
    else if (m[5] !== undefined) out.push({ t: 'bool', v: m[5] === 'TRUE' });
    else out.push({ t: 'op', v: m[6] });
  }
  return out;
}

// Precedence climbing -> AST.
export function parse(src) {
  const toks = tokenize(src);
  let i = 0;
  const peek = () => toks[i];
  const take = () => toks[i++];
  const isOp = (...ops) => peek()?.t === 'op' && ops.includes(peek().v);
  const LEVELS = [['=', '<>', '<', '>', '<=', '>='], ['&'], ['+', '-'], ['*', '/'], ['^']];
  function binary(level) {
    if (level === LEVELS.length) return unary();
    let left = binary(level + 1);
    while (isOp(...LEVELS[level])) { const op = take().v; left = { k: 'bin', op, a: left, b: binary(level + 1) }; }
    return left;
  }
  function unary() {
    if (isOp('-', '+')) { const op = take().v; return { k: 'un', op, a: unary() }; }
    let node = primary();
    while (isOp('%')) { take(); node = { k: 'bin', op: '/', a: node, b: { k: 'lit', v: 100 } }; }
    return node;
  }
  function primary() {
    const t = take();
    if (!t) err('#VALUE!');
    if (t.t === 'num' || t.t === 'str' || t.t === 'bool') return { k: 'lit', v: t.v };
    if (t.t === 'ref') return { k: 'ref', v: t.v };
    if (t.t === 'fn') {
      const args = [];
      if (!isOp(')')) { do args.push(binary(0)); while (isOp(',') && take()); }
      if (!isOp(')')) err('#VALUE!');
      take();
      return { k: 'fn', name: t.v, args };
    }
    if (t.t === 'op' && t.v === '(') { const n = binary(0); if (!isOp(')')) err('#VALUE!'); take(); return n; }
    return err('#VALUE!');
  }
  const ast = binary(0);
  if (i < toks.length) err('#VALUE!');
  return ast;
}

const num = (v) => {
  if (v === '' || v === null || v === undefined) return 0;
  if (typeof v === 'boolean') return v ? 1 : 0;
  const n = typeof v === 'number' ? v : Number(v);
  if (Number.isNaN(n)) err('#VALUE!');
  return n;
};
const flat = (args) => args.flat(Infinity);
const nums = (args) => flat(args).filter((v) => typeof v === 'number');

const FUNCS = {
  SUM: (a) => nums(a).reduce((x, y) => x + y, 0),
  AVERAGE: (a) => { const n = nums(a); return n.length ? n.reduce((x, y) => x + y, 0) / n.length : err('#DIV/0!'); },
  MIN: (a) => (nums(a).length ? Math.min(...nums(a)) : 0),
  MAX: (a) => (nums(a).length ? Math.max(...nums(a)) : 0),
  COUNT: (a) => nums(a).length,
  COUNTA: (a) => flat(a).filter((v) => v !== '').length,
  ABS: (a) => Math.abs(num(a[0])),
  ROUND: (a) => { const f = 10 ** num(a[1] ?? 0); return Math.round(num(a[0]) * f) / f; },
  SQRT: (a) => (num(a[0]) < 0 ? err('#NUM!') : Math.sqrt(num(a[0]))),
  POWER: (a) => num(a[0]) ** num(a[1]),
  MOD: (a) => (num(a[1]) === 0 ? err('#DIV/0!') : num(a[0]) % num(a[1])),
  IF: (a) => (a[0] ? (a[1] ?? true) : (a[2] ?? false)),
  AND: (a) => flat(a).every(Boolean),
  OR: (a) => flat(a).some(Boolean),
  NOT: (a) => !a[0],
  CONCAT: (a) => flat(a).join(''),
  LEN: (a) => String(a[0] ?? '').length,
  UPPER: (a) => String(a[0] ?? '').toUpperCase(),
  LOWER: (a) => String(a[0] ?? '').toLowerCase(),
  PI: () => Math.PI,
};
// IF must not evaluate the branch it does not take, so it is handled lazily below.

export class Sheet {
  constructor(cells = {}) { this.cells = cells; this.cache = new Map(); }

  set(a, raw) {
    if (raw === '' || raw == null) delete this.cells[a]; else this.cells[a] = String(raw);
    this.cache.clear();
  }
  raw(a) { return this.cells[a] ?? ''; }

  // Display-ready value of a cell: number, string, boolean or an error code string.
  value(a) {
    if (this.cache.has(a)) return this.cache.get(a);
    this.cache.set(a, { cycle: true });
    let v;
    try { v = this.#cell(a); } catch (e) { v = e instanceof FormulaError ? e.message : '#VALUE!'; }
    this.cache.set(a, v);
    return v;
  }

  #cell(a) {
    const raw = this.raw(a);
    if (raw === '') return '';
    if (raw[0] === '=') return this.#eval(parse(raw.slice(1)));
    if (raw.trim() !== '' && !Number.isNaN(Number(raw))) return Number(raw);
    if (/^(TRUE|FALSE)$/i.test(raw)) return raw.toUpperCase() === 'TRUE';
    return raw;
  }

  #ref(a) {
    const c = this.cache.get(a);
    if (c && typeof c === 'object' && c.cycle) err('#CYCLE!');
    const v = this.value(a);
    if (typeof v === 'string' && v.startsWith('#') && /^#[A-Z/0!?]+!?$/.test(v)) err(v);
    return v;
  }

  #range(s) {
    const [p, q] = s.split(':').map((x) => /^([A-Z]+)(\d+)$/.exec(x));
    if (!p || !q) err('#REF!');
    const [c1, c2] = [colIndex(p[1]), colIndex(q[1])].sort((x, y) => x - y);
    const [r1, r2] = [+p[2], +q[2]].sort((x, y) => x - y);
    if ((c2 - c1 + 1) * (r2 - r1 + 1) > 100000) err('#REF!');
    const rows = [];
    for (let r = r1; r <= r2; r++) { const row = []; for (let c = c1; c <= c2; c++) row.push(this.#ref(addr(c, r - 1))); rows.push(row); }
    return rows;
  }

  #eval(n) {
    switch (n.k) {
      case 'lit': return n.v;
      case 'ref': return n.v.includes(':') ? this.#range(n.v) : this.#ref(n.v);
      case 'un': return n.op === '-' ? -num(this.#eval(n.a)) : num(this.#eval(n.a));
      case 'bin': {
        const a = this.#eval(n.a), b = this.#eval(n.b);
        switch (n.op) {
          case '+': return num(a) + num(b);
          case '-': return num(a) - num(b);
          case '*': return num(a) * num(b);
          case '/': return num(b) === 0 ? err('#DIV/0!') : num(a) / num(b);
          case '^': return num(a) ** num(b);
          case '&': return `${a}${b}`;
          case '=': return a === b;
          case '<>': return a !== b;
          case '<': return a < b;
          case '>': return a > b;
          case '<=': return a <= b;
          default: return a >= b;
        }
      }
      case 'fn': {
        if (n.name === 'IF') {
          const cond = this.#eval(n.args[0]);
          const branch = cond ? n.args[1] : n.args[2];
          return branch ? this.#eval(branch) : !!cond;
        }
        const f = FUNCS[n.name];
        if (!f) err('#NAME?');
        return f(n.args.map((x) => this.#eval(x)));
      }
      default: return err('#VALUE!');
    }
  }
}

export function toCsv(sheet, cols, rows) {
  const out = [];
  for (let r = 0; r < rows; r++) {
    const line = [];
    for (let c = 0; c < cols; c++) {
      const v = String(sheet.value(addr(c, r)));
      line.push(/[",\n]/.test(v) ? `"${v.replace(/"/g, '""')}"` : v);
    }
    out.push(line.join(','));
  }
  return out.join('\n').replace(/(,*\n)+$/, '') + '\n';
}

export function fromCsv(text) {
  const cells = {};
  let r = 0, c = 0, field = '', quoted = false;
  const end = () => { if (field !== '') cells[addr(c, r)] = field; field = ''; };
  for (let i = 0; i < text.length; i++) {
    const ch = text[i];
    if (quoted) {
      if (ch === '"' && text[i + 1] === '"') { field += '"'; i++; } else if (ch === '"') quoted = false; else field += ch;
    } else if (ch === '"') quoted = true;
    else if (ch === ',') { end(); c++; }
    else if (ch === '\n' || ch === '\r') { if (ch === '\r' && text[i + 1] === '\n') i++; end(); r++; c = 0; }
    else field += ch;
  }
  end();
  return cells;
}
