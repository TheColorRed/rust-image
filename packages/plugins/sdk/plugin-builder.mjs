// Shared plugin builder used by both the package command and the live-sync watcher.
import chalk from 'chalk';
import { build as esbuild, transform as esbuildTransform } from 'esbuild';
import { createHash } from 'node:crypto';
import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import path from 'node:path';
import { parse } from 'yaml';

/** Starts esbuild's service before the first file change, keeping its startup time out of the edit cycle. */
export async function warmPluginBuilder() {
  await esbuildTransform('', { loader: 'ts' });
}

/** Bundles a TypeScript entry; app-provided modules remain external for Alakazam to resolve. */
async function bundle(srcDir, entry) {
  const pluginDir = path.dirname(srcDir);
  const entryPoint = `./${path.relative(pluginDir, path.join(srcDir, entry.slice(0, -'.js'.length))).split(path.sep).join('/')}`;
  const result = await esbuild({
    absWorkingDir: pluginDir,
    entryPoints: [entryPoint],
    bundle: true,
    write: false,
    platform: 'neutral',
    format: 'cjs',
    target: 'es2019',
    external: ['@abra/core', '@abra/core/*', '@alakazam/sdk'],
    logLevel: 'silent',
  });
  return withPerKeyExports(result.outputFiles[0].text);
}

/**
 * Hermes gives every closure created in esbuild's `__copyProps` loop (`for (let key of ...)`) the same `key`, so each export getter
 * returns the last export and `renderInspector` came back as `selectedImages`. Running the loop body once per key through `forEach`
 * gives every getter its own `key`.
 */
function withPerKeyExports(code) {
  const loop = /for \(let key of __getOwnPropNames\(from\)\)\n(\s+if \(!__hasOwnProp\.call\(to, key\) && key !== except\)\n\s+__defProp\(to, key, \{[^\n]*\}\);)/;
  if (!code.includes('for (let key of __getOwnPropNames(from))')) return code;
  if (!loop.test(code)) throw new Error('esbuild changed its export helper; update withPerKeyExports in plugin-builder.mjs');
  return code.replace(loop, '__getOwnPropNames(from).forEach((key) => {\n$1\n    });');
}

/** Entry JS files named by a plugin manifest, in manifest order. */
function entriesOf(manifest) {
  const controls = (manifest.contributes?.controls ?? []).flatMap(group => (group.items ?? []).map(item => item.entry));
  const scripts = (manifest.contributes?.scripts ?? []).map(script => script.entry);
  const workspaces = (manifest.contributes?.workspaces ?? []).map(workspace => workspace.entry);
  return [...new Set([...controls, ...scripts, ...workspaces])];
}

/** Replaces the manifest's files block with the built entries and their hashes. */
function withFiles(text, files) {
  const block = ['files:', ...files.flatMap(({ path: file, sha256 }) => [`  - path: ${file}`, `    sha256: ${sha256}`])].join('\n') + '\n';
  const lines = text.replace(/\r\n/g, '\n').split('\n');
  const start = lines.findIndex(line => line === 'files:');
  if (start < 0) return text.replace(/\r\n/g, '\n').replace(/\n*$/, '\n\n') + block;
  let end = start + 1;
  while (end < lines.length && (lines[end].startsWith(' ') || lines[end].startsWith('-'))) end++;
  return [...lines.slice(0, start), ...block.trimEnd().split('\n'), ...lines.slice(end)].join('\n');
}

export async function buildPlugins(pluginDirs, { devMode = false } = {}) {
  let built = 0;
  for (const dir of pluginDirs.map(value => path.resolve(value))) {
    const manifestPath = path.join(dir, 'plugin.yml');
    if (!existsSync(manifestPath)) throw new Error(`No plugin.yml found in ${dir}`);
    const original = readFileSync(manifestPath, 'utf8');
    const manifest = parse(original);
    if (typeof manifest?.id !== 'string' || !manifest.id) throw new Error(`${manifestPath} has no plugin id`);
    const outDir = path.join(dir, 'dist');
    const srcDir = path.join(dir, 'src');
    const builtFiles = [];
    for (const entry of entriesOf(manifest)) {
      if (typeof entry !== 'string' || !entry.endsWith('.js')) throw new Error(`${dir}: ${entry} is not a .js file`);
      const code = await bundle(srcDir, entry);
      const outputPath = path.join(outDir, entry);
      mkdirSync(path.dirname(outputPath), { recursive: true });
      writeFileSync(outputPath, code);
      builtFiles.push({ path: entry, sha256: createHash('sha256').update(code).digest('hex') });
      console.log(chalk.dim(`${path.basename(dir)}/${path.relative(dir, outputPath).split(path.sep).join('/')}  ${code.length} bytes`));
    }

    const builtPaths = new Set(builtFiles.map(file => file.path));
    const otherFiles = Array.isArray(manifest.files) ? manifest.files.filter(file => !builtPaths.has(file.path)) : [];
    const files = [...builtFiles, ...otherFiles];
    const listedPaths = new Set(files.map(file => file.path));
    for (const model of manifest.contributes?.models ?? []) {
      for (const file of [model.model, model.manifest].filter(value => typeof value === 'string' && value !== '')) {
        if (!listedPaths.has(file)) throw new Error(`${manifest.id} names ${file} but does not list it in files`);
      }
    }
    const updated = withFiles(original, files);
    const packageFiles = [];
    const included = new Set(['plugin.yml']);
    for (const file of files) {
      const filePath = file.path.split('/');
      if (included.has(file.path) || path.isAbsolute(file.path) || filePath.some(part => part === '' || part === '.' || part === '..') || file.path.includes('\\')) {
        throw new Error(`${file.path} is not a safe relative plugin file path`);
      }
      included.add(file.path);
      const builtPath = builtPaths.has(file.path);
      const sourcePath = path.join(builtPath ? outDir : dir, ...filePath);
      if (!existsSync(sourcePath)) throw new Error(`${file.path} is missing from ${builtPath ? 'dist' : 'the plugin folder'}`);
      const data = readFileSync(sourcePath);
      const sha256 = createHash('sha256').update(data).digest('hex');
      if (sha256 !== file.sha256) throw new Error(`${file.path} does not match its SHA-256 in plugin.yml`);
      packageFiles.push({ path: file.path, data });
    }
    if (!devMode) {
      const { default: JSZip } = await import('jszip');
      const zip = new JSZip();
      zip.file('plugin.yml', updated);
      for (const file of packageFiles) zip.file(file.path, file.data);
      const archive = await zip.generateAsync({ type: 'nodebuffer', compression: 'STORE' });
      const archivePath = path.join(outDir, `${manifest.id}.akzx`);
      writeFileSync(archivePath, archive);
      console.log(chalk.dim(`${path.basename(dir)}/${path.relative(dir, archivePath).split(path.sep).join('/')}  ${archive.length} bytes`));
    }
    if (updated !== original.replace(/\r\n/g, '\n')) writeFileSync(manifestPath, updated);
    built++;
  }
  console.log(chalk.bold(`${built} plugin(s) built${devMode ? ' for development sync' : ' and packaged'}.`));
}
