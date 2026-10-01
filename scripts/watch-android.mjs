// Redeploys the Android app whenever the Rust code behind it changes.
//
// Watches abra/ and the React Native binding crate, waits for edits to settle, then runs the same steps as
// `npm run deploy:android` (build the Rust libraries and bindings, install with Gradle, launch). An edit made during a
// build cancels it and starts over with the new code. JS/TS changes are not watched: Metro already reloads those.
//
// Usage: npm run watch:android [-- --release] [-- --no-launch]
import { spawn, spawnSync } from 'node:child_process';
import { watch } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const deployScript = path.join(root, 'scripts', 'build-android.mjs');
const deployArgs = process.argv.slice(2);

const WATCHED = ['abra', path.join('packages', 'react-native', 'alakazam', 'src')];
const SOURCE_EXTENSIONS = new Set(['.rs', '.wgsl', '.toml']);
/** Folders that are build output or written by the deploy itself; watching them would loop. */
const IGNORED = new Set(['target', 'generated', 'node_modules', '.git']);
const SETTLE_MS = 800;

let timer;
/** The deploy in progress, if any. */
let current = null;

function isSource(p_file) {
  const parts = p_file.split(path.sep);
  return !parts.some(part => IGNORED.has(part)) && SOURCE_EXTENSIONS.has(path.extname(p_file));
}

/** Stops the deploy and everything it started (npm, cargo, Gradle), not only the top process. */
function cancel(p_child) {
  p_child.cancelled = true;
  if (process.platform === 'win32') spawnSync('taskkill', ['/pid', String(p_child.pid), '/T', '/F'], { stdio: 'ignore' });
  else process.kill(-p_child.pid, 'SIGTERM');
}

function deploy() {
  console.log(`
[watch-android] ${new Date().toLocaleTimeString()} change detected, deploying...`);
  // Its own process group off Windows, so cancelling can signal the whole tree.
  const child = spawn(process.execPath, [deployScript, ...deployArgs], {
    cwd: root,
    stdio: 'inherit',
    detached: process.platform !== 'win32',
  });
  current = child;
  child.on('exit', code => {
    if (current === child) current = null;
    if (child.cancelled) return console.log('[watch-android] deploy cancelled by a newer change.');
    console.log(`[watch-android] deploy ${code === 0 ? 'finished' : `failed (exit ${code})`}; watching for changes.`);
  });
}

/** Called on every relevant save: cancels a running deploy at once, and starts a fresh one when edits settle. */
function schedule() {
  const running = current;
  if (running && !running.cancelled) cancel(running);
  clearTimeout(timer);
  timer = setTimeout(() => {
    // Wait for the cancelled deploy to be gone, so two builds never share the target directory.
    if (current) current.once('exit', deploy);
    else deploy();
  }, SETTLE_MS);
}

for (const dir of WATCHED) {
  watch(path.join(root, dir), { recursive: true }, (_event, file) => {
    if (file && isSource(file)) schedule();
  });
}
console.log(`[watch-android] watching ${WATCHED.join(', ')}. Run "npm run start:android" for Metro. Ctrl+C to stop.`);
