// Packs the plugins of the repository into folders that are ready to upload, one folder for each plugin, in `target/plugin-packages`.
//
// A plugin is found as `packages/abra/ai/<crate>/plugins/<folder>/plugin.yml`, or `apps/alakazam/mobile/plugins/<folder>/plugin.yml`. Its `files` are taken from the plugin's own folder, or when
// they are not there from the `models` folder of its crate, so a model that other code reads from `models` is not copied into the
// repository twice. Every file is checked against the SHA-256 the plugin lists, and the folder is written the way the bucket holds it:
// `plugins/<folder>/plugin.yml` with its files beside it. The folder's name is only where the plugin is kept: the plugin is named by the
// `id` in its plugin.yml, so the folder can be called anything and can be moved without the plugin knowing.
//
// It prints what to upload, and the lines to add to the catalog's `plugins:` list: the plugin.yml's URL and its SHA-256, which only a
// finished plugin.yml has. Usage: `node scripts/pack-plugins.mjs [folder ...]` (all plugins when none is named).
import { execFileSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { copyFileSync, existsSync, mkdirSync, readFileSync, readdirSync, rmSync, statSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { parse } from 'yaml';
import { outDirOf } from './plugin-config.mjs';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const aiDir = path.join(root, 'packages/abra/ai');
const appDir = path.join(root, 'apps/alakazam/mobile/plugins');
const outDir = path.join(root, 'target/plugin-packages');
const BUCKET_URL = 'https://alakazam-image-editor.s3.us-east-1.amazonaws.com/plugins';
const wanted = new Set(process.argv.slice(2));
const problems = [];
const packed = [];

const sha256 = (file) => createHash('sha256').update(readFileSync(file)).digest('hex');

const found = [];
const appFolders = existsSync(appDir) ? readdirSync(appDir) : [];
// Build app plugins before packing them. AI plugins are already built, so packing one does not rebuild every app plugin.
const appPlugins = appFolders.filter((folder) => existsSync(path.join(appDir, folder, 'plugin.yml')));
const appPluginsToBuild = wanted.size === 0 ? [] : [...wanted].filter((folder) => appPlugins.includes(folder));
if (wanted.size === 0 || appPluginsToBuild.length > 0) {
  execFileSync(process.execPath, [path.join(root, 'scripts/build-plugins.mjs'), ...appPluginsToBuild], { stdio: 'inherit' });
}

for (const folder of appPlugins) {
  const dir = path.join(appDir, folder);
  if (existsSync(path.join(dir, 'plugin.yml'))) found.push({ crate: null, folder, dir });
}
for (const crate of readdirSync(aiDir)) {
  const pluginsDir = path.join(aiDir, crate, 'plugins');
  if (!existsSync(pluginsDir)) continue;
  for (const folder of readdirSync(pluginsDir)) {
    const dir = path.join(pluginsDir, folder);
    if (existsSync(path.join(dir, 'plugin.yml'))) found.push({ crate, folder, dir });
  }
}

for (const folder of wanted) {
  if (!found.some((plugin) => plugin.folder === folder)) problems.push(`${folder}: no plugin folder with this name was found`);
}

for (const { crate, folder, dir } of found) {
  if (wanted.size > 0 && !wanted.has(folder)) continue;
  const manifest = parse(readFileSync(path.join(dir, 'plugin.yml'), 'utf8'));
  const out = path.join(outDir, folder);
  rmSync(out, { recursive: true, force: true });
  mkdirSync(out, { recursive: true });
  const files = Array.isArray(manifest.files) ? manifest.files : [];
  if (files.length === 0) problems.push(`${folder}: the plugin.yml lists no files, so nothing can be downloaded`);
  let ok = true;
  for (const file of files) {
    const from = [path.join(dir, file.path), path.join(outDirOf(dir), file.path), crate && path.join(aiDir, crate, 'models', file.path)]
      .filter(Boolean)
      .find(existsSync);
    if (!from) {
      problems.push(`${folder}: ${file.path} is not in the plugin's folder or in ${crate}/models`);
      ok = false;
      continue;
    }
    const actual = sha256(from);
    if (actual !== file.sha256) {
      problems.push(`${folder}: ${file.path} has the SHA-256 ${actual}, not the ${file.sha256} the plugin.yml lists`);
      ok = false;
      continue;
    }
    mkdirSync(path.dirname(path.join(out, file.path)), { recursive: true });
    copyFileSync(from, path.join(out, file.path));
  }
  if (!ok) continue;
  copyFileSync(path.join(dir, 'plugin.yml'), path.join(out, 'plugin.yml'));
  packed.push({ folder, out, files: files.map((file) => file.path), manifestSha256: sha256(path.join(out, 'plugin.yml')) });
}

for (const { folder, out, files, manifestSha256 } of packed) {
  const bytes = files.reduce((total, file) => total + statSync(path.join(out, file)).size, 0);
  console.log(`\n${folder}  (${(bytes / 1e6).toFixed(1)} MB)  ${path.relative(root, out)}`);
  for (const file of ['plugin.yml', ...files]) console.log(`  plugins/${folder}/${file}`);
  console.log(`  catalog:  - uri: ${BUCKET_URL}/${folder}/plugin.yml\n            sha256: ${manifestSha256}`);
}
if (problems.length > 0) {
  console.error(`\n${problems.length} problem(s):\n${problems.map((problem) => `  ${problem}`).join('\n')}`);
  process.exit(1);
}
console.log(`\n${packed.length} plugin(s) packed.`);
