import glob from 'fast-glob';
import { rename } from 'fs/promises';
import { basename, dirname, join, resolve } from 'path';
import { fileURLToPath } from 'url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

const bindings = 'packages/node/*/tmp/*.{node,d.ts}';
const cwd = resolve(join(__dirname, '../../..'));
const files = await glob(bindings, { cwd });
let movedFiles = 0;

// Move each file from tmp up one directory level
for (const file of files) {
  const filename = basename(file);
  // Move file from tmp/filename to filename
  const dest = join(dirname(file), '..', filename);
  await rename(join(cwd, file), join(cwd, dest));
  console.log(`Moved: ${file} -> ${dest}`);
  movedFiles++;
}

// intentionally do not write a sentinel here; the build writes the sentinel when bindings are ready
if (movedFiles === 0) console.log('No binding files to move.');
