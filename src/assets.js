// Choosing the right release file for this computer, and reading checksum files.
const OS_WORDS = {
  win32: ['windows', 'win', 'win64', 'msi', 'exe'],
  darwin: ['macos', 'mac', 'osx', 'darwin', 'dmg'],
  linux: ['linux', 'appimage', 'deb', 'rpm'],
};
const ARCH_WORDS = {
  x64: ['x64', 'x86_64', 'amd64'],
  arm64: ['arm64', 'aarch64'],
};
const KIND_ORDER = {
  win32: ['.msi', '.exe'],
  darwin: ['.dmg', '.pkg'],
  linux: ['.appimage', '.deb', '.rpm', '.tar.gz'],
};

export function currentPlatform(proc = process) {
  return { os: proc.platform, arch: proc.arch };
}

const words = (name) => name.toLowerCase().split(/[^a-z0-9_]+/).filter(Boolean);
const hasAny = (ws, list) => list.some((w) => ws.includes(w));

// Best asset for {os, arch}, or null. Checksum and signature files are never candidates.
export function pickAsset(assets, { os, arch }) {
  const kinds = KIND_ORDER[os];
  if (!kinds || !Array.isArray(assets)) return null;
  const mine = ARCH_WORDS[arch] ?? [];
  const otherArch = Object.entries(ARCH_WORDS).filter(([a]) => a !== arch).flatMap(([, w]) => w);
  let best = null;
  let bestScore = 0;
  for (const a of assets) {
    const name = String(a?.name ?? '');
    const lower = name.toLowerCase();
    const kind = kinds.findIndex((k) => lower.endsWith(k));
    if (kind < 0 || /sha256|\.sig$|\.asc$|\.sbom/.test(lower)) continue;
    const ws = words(name);
    if (hasAny(ws, otherArch) && !hasAny(ws, mine)) continue; // built for another CPU
    let score = 10 - kind;
    if (hasAny(ws, mine)) score += 20;
    if (hasAny(ws, OS_WORDS[os])) score += 5;
    if (score > bestScore) {
      best = a;
      bestScore = score;
    }
  }
  return best;
}

// "<64 hex>  filename" lines -> Map(filename -> hex). Ignores malformed lines.
export function parseSha256Sums(text) {
  const out = new Map();
  for (const line of String(text ?? '').split(/\r?\n/)) {
    const m = /^([0-9a-fA-F]{64})\s+\*?(.+?)\s*$/.exec(line);
    if (m) out.set(m[2], m[1].toLowerCase());
  }
  return out;
}
