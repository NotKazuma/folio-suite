// Tolerant semver: "v1.2.3", "1.2", "1.2.3-beta.1". Returns null when unparseable.
export function parseVersion(text) {
  const m = /^v?(\d+)(?:\.(\d+))?(?:\.(\d+))?(?:-([0-9A-Za-z.-]+))?/.exec(String(text ?? '').trim());
  if (!m) return null;
  return { major: +m[1], minor: +(m[2] ?? 0), patch: +(m[3] ?? 0), pre: m[4] ?? null };
}

// Returns -1, 0 or 1; unparseable versions sort lowest.
export function compareVersions(a, b) {
  const x = parseVersion(a), y = parseVersion(b);
  if (!x || !y) return x ? 1 : y ? -1 : 0;
  for (const k of ['major', 'minor', 'patch']) if (x[k] !== y[k]) return x[k] < y[k] ? -1 : 1;
  if (x.pre === y.pre) return 0;
  if (x.pre === null) return 1; // a release outranks its pre-releases
  if (y.pre === null) return -1;
  return x.pre < y.pre ? -1 : 1;
}

export const isNewer = (latest, installed) => compareVersions(latest, installed) > 0;
