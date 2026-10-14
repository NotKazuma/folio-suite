const grid = document.getElementById('grid');

const LABEL = {
  'not-installed': 'Install',
  update: 'Update',
  current: 'Reinstall',
};

function el(tag, props = {}, ...kids) {
  const n = Object.assign(document.createElement(tag), props);
  for (const k of kids) n.append(k);
  return n;
}

function card(app) {
  const working = app.job?.state === 'working';
  const tile = el('div', { className: 'tile' }, app.name.replace('Folio ', '').slice(0, 2));
  tile.style.background = app.color;
  const meta = [];
  if (app.installed) meta.push(`Installed ${app.installed}`);
  if (app.latest) meta.push(`Latest ${app.latest}`);
  const note = { unreleased: 'No release yet.', 'no-build': 'No build for this computer yet.', unknown: app.error ?? 'Status unknown.' }[app.status];
  const children = [
    tile,
    el('h2', {}, app.name),
    el('p', { className: 'tagline' }, `${app.tagline} · an alternative to ${app.replaces}`),
    el('p', { className: 'meta' }, meta.join(' · ') || note || ''),
  ];
  if (app.job?.state === 'failed') children.push(el('p', { className: 'meta err' }, app.job.error));
  if (working) children.push(Object.assign(el('progress'), { max: 100, value: app.job.percent }));
  const row = el('div', { className: 'row' });
  if (LABEL[app.status] && app.downloadable) {
    const b = el('button', { textContent: working ? 'Working…' : LABEL[app.status], disabled: working });
    b.onclick = () => start(app.id);
    row.append(b);
  }
  if (app.web) row.append(el('a', { href: app.web, className: 'open' }, 'Open in browser'));
  row.append(el('a', { href: app.releaseUrl, target: '_blank', rel: 'noopener' }, 'Release page'));
  children.push(row);
  return el('article', { className: 'card' }, ...children);
}

async function refresh() {
  try {
    const apps = await (await fetch('/api/apps')).json();
    grid.replaceChildren(...apps.map(card));
    return apps.some((a) => a.job?.state === 'working');
  } catch {
    grid.textContent = 'Could not reach the Folio Hub service.';
    return false;
  }
}

async function start(id) {
  await fetch(`/api/install/${id}`, { method: 'POST', headers: { 'X-Folio': '1' } });
  poll();
}

async function poll() {
  while (await refresh()) await new Promise((r) => setTimeout(r, 800));
}

poll();
