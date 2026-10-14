import { createWriteStream } from 'node:fs';
import { mkdir, unlink } from 'node:fs/promises';
import { createHash } from 'node:crypto';
import path from 'node:path';
import { Readable } from 'node:stream';
import { parseSha256Sums } from './assets.js';

export const MAX_BYTES = 1.5 * 1024 ** 3;
const CDN_HOSTS = ['objects.githubusercontent.com', 'release-assets.githubusercontent.com'];

// https only; github.com URLs must sit under the suite's org; GitHub's asset CDN is allowed.
export function assertAllowedUrl(url, org) {
  const u = new URL(url);
  const own = u.hostname === 'github.com' && u.pathname.toLowerCase().startsWith(`/${org.toLowerCase()}/`);
  if (u.protocol !== 'https:' || !(own || CDN_HOSTS.includes(u.hostname))) {
    throw new Error(`Refusing to download from ${u.hostname}.`);
  }
}

// Streams url to dir/name while hashing. Deletes the file and throws on a size or checksum failure.
export async function downloadVerified({ url, name, dir, expectedSha, org, onProgress, fetchImpl = fetch }) {
  assertAllowedUrl(url, org);
  if (path.basename(name) !== name) throw new Error('Unsafe file name.');
  await mkdir(dir, { recursive: true });
  const dest = path.join(dir, name);
  const res = await fetchImpl(url, { headers: { 'User-Agent': 'folio-hub' }, redirect: 'follow' });
  if (!res.ok || !res.body) throw new Error(`Download failed (${res.status}).`);
  if (res.url) assertAllowedUrl(res.url, org); // the final hop after redirects
  const total = Number(res.headers.get('content-length')) || 0;
  if (total > MAX_BYTES) throw new Error('File is larger than the allowed limit.');
  const hash = createHash('sha256');
  const out = createWriteStream(dest);
  let got = 0;
  try {
    for await (const chunk of Readable.fromWeb(res.body)) {
      got += chunk.length;
      if (got > MAX_BYTES) throw new Error('File is larger than the allowed limit.');
      hash.update(chunk);
      if (!out.write(chunk)) await new Promise((r) => out.once('drain', r));
      onProgress?.(got, total);
    }
    await new Promise((resolve, reject) => out.end((e) => (e ? reject(e) : resolve())));
    const actual = hash.digest('hex');
    if (expectedSha && actual !== expectedSha) throw new Error('Checksum mismatch: the file was not kept.');
    return { path: dest, sha256: actual, verified: Boolean(expectedSha) };
  } catch (e) {
    out.destroy();
    await unlink(dest).catch(() => {});
    throw e;
  }
}

export async function fetchChecksums(release, fetchImpl = fetch) {
  const sums = release.assets.find((a) => /^sha256sums(\.txt)?$/i.test(a.name));
  if (!sums) return new Map();
  const res = await fetchImpl(sums.url, { headers: { 'User-Agent': 'folio-hub' } });
  return res.ok ? parseSha256Sums((await res.text()).slice(0, 1_000_000)) : new Map();
}
