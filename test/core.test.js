import test from 'node:test';
import assert from 'node:assert/strict';
import { compareVersions, isNewer, parseVersion } from '../src/version.js';
import { pickAsset, parseSha256Sums } from '../src/assets.js';
import { assertAllowedUrl } from '../src/download.js';
import { statusOf } from '../src/releases.js';

test('versions compare numerically, not as text', () => {
  assert.equal(compareVersions('v1.10.0', '1.9.9'), 1);
  assert.equal(compareVersions('1.0.0', '1.0.0-beta.1'), 1);
  assert.equal(compareVersions('1.2', '1.2.0'), 0);
  assert.equal(parseVersion('nonsense'), null);
  assert.ok(isNewer('2.0.0', '1.9.0'));
});

const assets = [
  { name: 'folio-write-1.0.0-windows-x64.msi' },
  { name: 'folio-write-1.0.0-windows-arm64.msi' },
  { name: 'folio-write-1.0.0-macos-arm64.dmg' },
  { name: 'folio-write-1.0.0-linux-x86_64.AppImage' },
  { name: 'folio-write-1.0.0-linux-x86_64.deb' },
  { name: 'SHA256SUMS.txt' },
];

test('picks the build for this OS and CPU', () => {
  assert.equal(pickAsset(assets, { os: 'win32', arch: 'x64' }).name, 'folio-write-1.0.0-windows-x64.msi');
  assert.equal(pickAsset(assets, { os: 'win32', arch: 'arm64' }).name, 'folio-write-1.0.0-windows-arm64.msi');
  assert.equal(pickAsset(assets, { os: 'darwin', arch: 'arm64' }).name, 'folio-write-1.0.0-macos-arm64.dmg');
  assert.equal(pickAsset(assets, { os: 'darwin', arch: 'x64' }), null);
  assert.equal(pickAsset(assets, { os: 'linux', arch: 'x64' }).name, 'folio-write-1.0.0-linux-x86_64.AppImage');
  assert.equal(pickAsset(assets, { os: 'freebsd', arch: 'x64' }), null);
});

test('reads checksum files and skips junk lines', () => {
  const h = 'a'.repeat(64);
  const m = parseSha256Sums(`${h}  one.msi\nnot a hash line\n${h.toUpperCase()} *two.dmg\n`);
  assert.equal(m.get('one.msi'), h);
  assert.equal(m.get('two.dmg'), h);
  assert.equal(m.size, 2);
});

test('only the suite org and GitHub asset hosts may be downloaded from', () => {
  assertAllowedUrl('https://github.com/folio-suite/folio-write/releases/download/v1/a.msi', 'folio-suite');
  assertAllowedUrl('https://objects.githubusercontent.com/x', 'folio-suite');
  assert.throws(() => assertAllowedUrl('https://github.com/evil/app/releases/download/v1/a.msi', 'folio-suite'));
  assert.throws(() => assertAllowedUrl('http://github.com/folio-suite/a', 'folio-suite'));
  assert.throws(() => assertAllowedUrl('https://example.com/a.msi', 'folio-suite'));
});

test('status reflects installed vs latest', () => {
  assert.equal(statusOf(null, { version: '1.0.0' }), 'not-installed');
  assert.equal(statusOf('1.0.0', { version: '1.1.0' }), 'update');
  assert.equal(statusOf('1.1.0', { version: '1.1.0' }), 'current');
  assert.equal(statusOf(null, { none: true }), 'unreleased');
  assert.equal(statusOf('1.0.0', { error: 'x' }), 'unknown');
});
