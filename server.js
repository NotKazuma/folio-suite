import http from 'node:http';
import { readFile } from 'node:fs/promises';
import { spawn } from 'node:child_process';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { currentPlatform, pickAsset } from './src/assets.js';
import { latestRelease, repoFor, statusOf } from './src/releases.js';
import { downloadVerified, fetchChecksums } from './src/download.js';
import { dataDir, loadState, saveState } from './src/state.js';

const here = path.dirname(fileURLToPath(import.meta.url));
const catalog = JSON.parse(await readFile(path.join(here, 'catalog.json'), 'utf8'));
const PORT = Number(process.env.PORT) || 4873;
const platform = currentPlatform();
const jobs = new Map(); // app id -> { state, percent, error }

const STATIC = {
  '/index.html': ['index.html', 'text/html; charset=utf-8'],
  '/': ['index.html', 'text/html; charset=utf-8'],
  '/app.js': ['app.js', 'text/javascript; charset=utf-8'],
  '/style.css': ['style.css', 'text/css; charset=utf-8'],
};

const MIME = { '.html': 'text/html; charset=utf-8', '.js': 'text/javascript; charset=utf-8', '.css': 'text/css; charset=utf-8' };

// Serves the bundled web apps under /apps/. Only .html/.js/.css files inside apps/ are reachable.
async function serveApp(res, pathname) {
  let rel = decodeURIComponent(pathname).replace(/^\/apps\//, '');
  if (rel === '' || rel.endsWith('/')) rel += 'index.html';
  const file = path.resolve(here, 'apps', rel);
  const type = MIME[path.extname(file)];
  if (!type || !file.startsWith(path.join(here, 'apps') + path.sep)) return json(res, 404, { error: 'not found' });
  try {
    res.writeHead(200, { 'Content-Type': type, 'Cache-Control': 'no-store' });
    res.end(await readFile(file));
  } catch { json(res, 404, { error: 'not found' }); }
}

const json = (res, code, body) => {
  res.writeHead(code, { 'Content-Type': 'application/json', 'Cache-Control': 'no-store' });
  res.end(JSON.stringify(body));
};

async function listApps() {
  const state = await loadState();
  return Promise.all(catalog.apps.map(async (app) => {
    const latest = await latestRelease(repoFor(catalog, app));
    const installed = state.installed[app.id]?.version ?? null;
    const asset = latest.assets ? pickAsset(latest.assets, platform) : null;
    return {
      ...app,
      web: app.web ? `/apps/${app.web}/` : null,
      installed,
      latest: latest.version ?? null,
      notes: latest.notes ?? '',
      releaseUrl: latest.url ?? `https://github.com/${repoFor(catalog, app)}/releases`,
      error: latest.error ?? null,
      downloadable: Boolean(asset),
      status: latest.assets && !asset ? 'no-build' : statusOf(installed, latest),
      job: jobs.get(app.id) ?? null,
    };
  }));
}

// Opens the verified installer with the OS's own handler; the user finishes the install there.
function openInstaller(file) {
  const [cmd, args] = platform.os === 'win32' ? ['cmd', ['/c', 'start', '""', file]]
    : platform.os === 'darwin' ? ['open', [file]] : ['xdg-open', [file]];
  spawn(cmd, args, { detached: true, stdio: 'ignore' }).unref();
}

async function install(app) {
  const job = { state: 'working', percent: 0, error: null };
  jobs.set(app.id, job);
  try {
    const release = await latestRelease(repoFor(catalog, app));
    if (!release.assets) throw new Error(release.error ?? 'No release has been published yet.');
    const asset = pickAsset(release.assets, platform);
    if (!asset) throw new Error('This release has no build for your computer.');
    const sums = await fetchChecksums(release);
    const expectedSha = sums.get(asset.name);
    const file = await downloadVerified({
      url: asset.url, name: asset.name, dir: path.join(dataDir, 'downloads'), expectedSha, org: catalog.org,
      onProgress: (got, total) => { job.percent = total ? Math.round((got / total) * 100) : 0; },
    });
    openInstaller(file.path);
    const state = await loadState();
    state.installed[app.id] = { version: release.version, verified: file.verified, at: new Date().toISOString() };
    await saveState(state);
    job.state = 'done';
    job.percent = 100;
  } catch (e) {
    job.state = 'failed';
    job.error = e.message;
  }
}

const server = http.createServer(async (req, res) => {
  // Local-only: refuse other Host headers (DNS rebinding) and cross-site writes.
  const host = (req.headers.host ?? '').split(':')[0];
  if (host !== '127.0.0.1' && host !== 'localhost') return json(res, 403, { error: 'forbidden' });
  const url = new URL(req.url, 'http://localhost');
  try {
    if (req.method === 'GET' && STATIC[url.pathname]) {
      const [file, type] = STATIC[url.pathname];
      res.writeHead(200, { 'Content-Type': type, 'Cache-Control': 'no-store' });
      return res.end(await readFile(path.join(here, 'public', file)));
    }
    if (req.method === 'GET' && url.pathname.startsWith('/apps/')) return serveApp(res, url.pathname);
    if (req.method === 'GET' && url.pathname === '/api/apps') return json(res, 200, await listApps());
    const m = /^\/api\/install\/([a-z0-9-]+)$/.exec(url.pathname);
    if (req.method === 'POST' && m) {
      if (req.headers['x-folio'] !== '1') return json(res, 403, { error: 'forbidden' });
      const app = catalog.apps.find((a) => a.id === m[1]);
      if (!app) return json(res, 404, { error: 'unknown app' });
      if (jobs.get(app.id)?.state === 'working') return json(res, 409, { error: 'already running' });
      install(app);
      return json(res, 202, { started: true });
    }
    json(res, 404, { error: 'not found' });
  } catch (e) {
    json(res, 500, { error: e.message });
  }
});

server.listen(PORT, '127.0.0.1', () => console.log(`Folio Hub: http://127.0.0.1:${PORT}`));
