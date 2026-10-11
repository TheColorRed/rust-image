// Puts NASM in tools/nasm unless it is already on the PATH. rav1d (the AVIF decoder) assembles its x86 code with it
// when it builds, and nothing is installed on the machine itself.
//   Windows:     downloads the portable build.
//   Linux/macOS: NASM publishes no portable build for them (the macOS one has no Apple Silicon code), so this builds
//                it from the source release, which needs a C compiler and make.
//
// Usage: runs after `npm install`, or by hand with `node scripts/install-nasm.mjs`
import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const version = '3.02';
const base = `https://www.nasm.us/pub/nasm/releasebuilds/${version}`;
const windows = process.platform === 'win32';
const binary = windows ? 'nasm.exe' : 'nasm';
const release = windows
  ? { url: `${base}/win64/nasm-${version}-win64.zip`, sha256: '161D0BFAFF53C2F9E9F3E69FD0672323EBABAFD1268976A5CEC11BE92A19AEE7' }
  : { url: `${base}/nasm-${version}.tar.gz`, sha256: 'F504227B2F529E658D41629075F0503B38D67D790AF345F34EBA4AF60C6A5998' };

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const destination = path.join(root, 'tools', 'nasm');

if (spawnSync('nasm', ['-v'], { stdio: 'ignore' }).status === 0) {
  console.log('install-nasm: nasm is already installed.');
  process.exit(0);
}
if (fs.existsSync(path.join(destination, binary))) {
  console.log(`install-nasm: tools/nasm/${binary} is already there.`);
  process.exit(0);
}

/** Runs a command and throws when it fails. */
function run(p_command, p_args, p_options = {}) {
  const result = spawnSync(p_command, p_args, { stdio: 'inherit', ...p_options });
  if (result.status !== 0) throw new Error(`"${p_command} ${p_args.join(' ')}" failed.`);
}

const work = fs.mkdtempSync(path.join(os.tmpdir(), 'nasm-'));
try {
  const response = await fetch(release.url);
  if (!response.ok) throw new Error(`Download failed: ${response.status} ${release.url}`);
  const archive = Buffer.from(await response.arrayBuffer());
  if (createHash('sha256').update(archive).digest('hex').toUpperCase() !== release.sha256) {
    throw new Error('The downloaded NASM does not match the expected checksum.');
  }
  const archivePath = path.join(work, path.basename(release.url));
  fs.writeFileSync(archivePath, archive);

  // Windows 10 and later ship a tar that reads zip files. Called by full path because Git Bash puts a GNU tar first,
  // and that one cannot read a drive letter.
  const tar = windows ? path.join(process.env.SystemRoot ?? 'C:\\Windows', 'System32', 'tar.exe') : 'tar';
  run(tar, ['-xf', archivePath, '-C', work]);

  const folder = path.join(work, `nasm-${version}`);
  if (!windows) {
    run('./configure', [], { cwd: folder });
    run('make', ['nasm'], { cwd: folder });
  }

  fs.mkdirSync(destination, { recursive: true });
  for (const file of [binary, 'LICENSE']) fs.copyFileSync(path.join(folder, file), path.join(destination, file));
  console.log(`install-nasm: installed NASM ${version} in tools/nasm.`);
} catch (error) {
  // Do not fail `npm install` over this: only builds that decode AVIF on x86 need it.
  console.warn(`install-nasm: ${error.message}\nRun "node scripts/install-nasm.mjs" to try again.`);
} finally {
  fs.rmSync(work, { recursive: true, force: true });
}
