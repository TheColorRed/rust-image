import { mkdir, writeFile } from 'fs/promises';
import { join, resolve } from 'path';

const cwd = resolve(join(process.cwd()));
const sentinelPath = join(cwd, 'apps', 'alakazam', '.bindings-ready.txt');
await mkdir(join(cwd, 'apps', 'alakazam'), { recursive: true });
await writeFile(sentinelPath, `bindings ready: ${new Date().toISOString()}\n`);
console.log('Wrote bindings-ready sentinel:', sentinelPath);
