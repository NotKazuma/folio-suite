import { isNewer } from './version.js';

const API = 'https://api.github.com';
const MAX_JSON = 2 * 1024 * 1024;
const cache = new Map(); // repo -> { etag, data }

// Only the suite's own GitHub org is ever contacted.
export const repoFor = (catalog, app) => `${catalog.org}/${app.id}`;

// Latest published release of repo, or { none: true } / { error }. ETags spare the rate limit.
export async function latestRelease(repo, fetchImpl = fetch) {
  const prev = cache.get(repo);
  const headers = { Accept: 'application/vnd.github+json', 'User-Agent': 'folio-hub' };
  if (process.env.GITHUB_TOKEN) headers.Authorization = `Bearer ${process.env.GITHUB_TOKEN}`;
  if (prev?.etag) headers['If-None-Match'] = prev.etag;
  try {
    const res = await fetchImpl(`${API}/repos/${repo}/releases/latest`, { headers });
    if (res.status === 304 && prev) return prev.data;
    if (res.status === 404) return { none: true };
    if (res.status === 403 || res.status === 429) return { error: 'GitHub rate limit reached; try again later.' };
    if (!res.ok) return { error: `GitHub answered ${res.status}.` };
    const text = await res.text();
    if (text.length > MAX_JSON) return { error: 'GitHub sent an unexpectedly large answer.' };
    const j = JSON.parse(text);
    const data = {
      version: String(j.tag_name ?? ''),
      notes: String(j.body ?? '').slice(0, 4000),
      url: String(j.html_url ?? ''),
      published: j.published_at ?? null,
      assets: (Array.isArray(j.assets) ? j.assets : []).map((a) => ({
        name: String(a.name),
        url: String(a.browser_download_url),
        size: Number(a.size) || 0,
      })),
    };
    cache.set(repo, { etag: res.headers.get('etag'), data });
    return data;
  } catch (e) {
    return prev?.data ?? { error: `Could not reach GitHub (${e.message}).` };
  }
}

export function statusOf(installed, latest) {
  if (!latest || latest.error) return 'unknown';
  if (latest.none) return 'unreleased';
  if (!installed) return 'not-installed';
  return isNewer(latest.version, installed) ? 'update' : 'current';
}
