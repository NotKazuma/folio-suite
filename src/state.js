import { mkdir, readFile, writeFile, rename } from 'node:fs/promises';
import { homedir } from 'node:os';
import path from 'node:path';

export const dataDir = process.env.FOLIO_HUB_HOME || path.join(homedir(), '.folio-hub');
const file = () => path.join(dataDir, 'state.json');

// Installed-apps record. A missing or corrupt file means "nothing recorded", never a crash.
export async function loadState() {
  try {
    const s = JSON.parse(await readFile(file(), 'utf8'));
    return s && typeof s.installed === 'object' && s.installed ? s : { installed: {} };
  } catch {
    return { installed: {} };
  }
}

export async function saveState(state) {
  await mkdir(dataDir, { recursive: true });
  const tmp = file() + '.tmp';
  await writeFile(tmp, JSON.stringify(state, null, 2));
  await rename(tmp, file());
}
