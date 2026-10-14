// Command registry shared by the Folio apps. Every action is a named command, so the toolbar,
// keyboard shortcuts and the command palette (Ctrl/Cmd+K) all run the same code.
const commands = new Map();

export function register(id, title, run, key = '') {
  commands.set(id, { id, title, run, key });
}

export function run(id, arg) {
  const c = commands.get(id);
  if (c) c.run(arg);
}

export const list = () => [...commands.values()];

// Wire data-cmd="id" buttons and the palette.
export function mount(root = document) {
  root.addEventListener('click', (e) => {
    const b = e.target.closest('[data-cmd]');
    if (b) run(b.dataset.cmd, b.dataset.arg);
  });
  root.addEventListener('keydown', (e) => {
    const mod = e.ctrlKey || e.metaKey;
    if (mod && e.key.toLowerCase() === 'k') { e.preventDefault(); openPalette(); return; }
    for (const c of commands.values()) {
      if (c.key && mod && e.key.toLowerCase() === c.key && !e.shiftKey && !e.altKey) { e.preventDefault(); c.run(); return; }
    }
  });
}

function openPalette() {
  const dlg = document.createElement('dialog');
  dlg.className = 'palette';
  dlg.innerHTML = '<input placeholder="Type a command…" aria-label="Command"><ul></ul>';
  const input = dlg.querySelector('input');
  const ul = dlg.querySelector('ul');
  let sel = 0, shown = [];
  const draw = () => {
    const q = input.value.toLowerCase();
    shown = list().filter((c) => c.title.toLowerCase().includes(q));
    sel = Math.min(sel, Math.max(0, shown.length - 1));
    ul.replaceChildren(...shown.map((c, i) => {
      const li = document.createElement('li');
      if (i === sel) li.className = 'sel';
      li.append(c.title);
      if (c.key) { const s = document.createElement('small'); s.textContent = 'Ctrl+' + c.key.toUpperCase(); li.append(s); }
      li.onclick = () => { dlg.close(); c.run(); };
      return li;
    }));
  };
  input.oninput = () => { sel = 0; draw(); };
  input.onkeydown = (e) => {
    if (e.key === 'ArrowDown') { sel = Math.min(sel + 1, shown.length - 1); draw(); e.preventDefault(); }
    else if (e.key === 'ArrowUp') { sel = Math.max(sel - 1, 0); draw(); e.preventDefault(); }
    else if (e.key === 'Enter') { const c = shown[sel]; dlg.close(); c?.run(); }
  };
  dlg.onclose = () => dlg.remove();
  document.body.append(dlg);
  draw();
  dlg.showModal();
  input.focus();
}

export function download(name, text, type = 'text/plain') {
  const a = document.createElement('a');
  a.href = URL.createObjectURL(new Blob([text], { type }));
  a.download = name;
  a.click();
  setTimeout(() => URL.revokeObjectURL(a.href), 1000);
}

export function pickFile(accept) {
  return new Promise((resolve) => {
    const i = document.createElement('input');
    i.type = 'file';
    i.accept = accept;
    i.onchange = async () => resolve(i.files[0] ? { name: i.files[0].name, text: await i.files[0].text() } : null);
    i.click();
  });
}

// localStorage that never throws (private windows, blocked storage).
export const store = {
  get(k) { try { return localStorage.getItem(k); } catch { return null; } },
  set(k, v) { try { localStorage.setItem(k, v); } catch { /* ignore */ } },
};
