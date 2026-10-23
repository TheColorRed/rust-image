// Builds the plugins of the Alakazam app (`apps/alakazam/mobile/plugins/<folder>`): turns the TypeScript of each script and control into the single
// JavaScript file that the plugin.yml names, and writes the `files:` list of the plugin.yml with the SHA-256 of what it built.
//
// A plugin's `entry: lipstick.js` is built from `src/lipstick.ts`, into the `outDir` of the plugin's tsconfig.json (`dist`), and is
// still named `lipstick.js` in the plugin.yml, which names the files as they are delivered. The files it imports with a relative path (`./lips`) are built into the
// same file, so an entry stands alone; everything else it imports (`@abra/core/color`, `@alakazam/host`) is left for the app to hand it
// when it runs (see `lib/plugins/plugin-host.ts`). The output is plain CommonJS and does not change from one build to the next, so the
// hashes in the plugin.yml only change when the source does.
//
// Each plugin is type-checked first with its own tsconfig.json (which extends `apps/alakazam/mobile/plugins/tsconfig.base.json`), so a plugin that
// does not type-check, or imports something the app does not offer plugins, is not built.
//
// Usage: `node scripts/build-plugins.mjs [folder ...]` (all plugins when none is named).
import { execFileSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { existsSync, mkdirSync, readFileSync, readdirSync, writeFileSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import ts from 'typescript';
import { parse } from 'yaml';
import { outDirOf } from './plugin-config.mjs';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const pluginsDir = path.join(root, 'apps/alakazam/mobile/plugins');
const wanted = new Set(process.argv.slice(2));

/** The source file for a relative import from `from` (both are ids: paths under `src` without an extension). */
function resolveId(srcDir, from, request) {
  const id = path.posix.normalize(path.posix.join(path.posix.dirname(from), request));
  for (const candidate of [id, `${id}/index`]) {
    if (existsSync(path.join(srcDir, `${candidate}.ts`))) return candidate;
  }
  throw new Error(`${from} imports ${request}, which is not in src`);
}

/** Every file an entry needs, itself included, by id, with the TypeScript turned into CommonJS. */
function collect(srcDir, entryId) {
  const modules = new Map();
  const visit = (id) => {
    if (modules.has(id)) return;
    const source = readFileSync(path.join(srcDir, `${id}.ts`), 'utf8');
    const output = ts.transpileModule(source, {
      fileName: `${id}.ts`,
      compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2019, esModuleInterop: false },
    });
    modules.set(id, output.outputText);
    for (const { fileName } of ts.preProcessFile(source, true, false).importedFiles) {
      if (fileName.startsWith('.')) visit(resolveId(srcDir, id, fileName));
    }
  };
  visit(entryId);
  return modules;
}

/** One file: a small module table, and a `require` that finds the files of the plugin and hands everything else to the app's own. */
function bundle(srcDir, entryId) {
  const modules = collect(srcDir, entryId);
  const defs = [...modules]
    .sort(([a], [b]) => a.localeCompare(b))
    .map(([id, code]) => `${JSON.stringify(id)}: function (module, exports, require) {\n${code.trimEnd()}\n}`)
    .join(',\n');
  return `"use strict";
var __defs = {
${defs}
};
var __cache = {};
function __join(from, request) {
  var parts = from.split("/").slice(0, -1).concat(request.split("/")), out = [];
  for (var i = 0; i < parts.length; i++) {
    if (parts[i] === "..") out.pop();
    else if (parts[i] !== "." && parts[i] !== "") out.push(parts[i]);
  }
  return out.join("/");
}
function __load(id) {
  if (__cache[id]) return __cache[id].exports;
  var module = (__cache[id] = { exports: {} });
  __defs[id](module, module.exports, function (request) {
    if (request.charAt(0) !== ".") return require(request);
    var target = __join(id, request);
    return __load(__defs[target] ? target : target + "/index");
  });
  return module.exports;
}
module.exports = __load(${JSON.stringify(entryId)});
`;
}

/** The scripts a plugin.yml names, in order: its controls' entries and its scripts' entries. */
function entriesOf(manifest) {
  const controls = (manifest.contributes?.controls ?? []).flatMap((group) => (group.items ?? []).map((item) => item.entry));
  const scripts = (manifest.contributes?.scripts ?? []).map((script) => script.entry);
  return [...new Set([...controls, ...scripts])];
}

/** Writes the `files:` list into the plugin.yml text, replacing the one that is there and leaving the rest as it is. */
function withFiles(text, files) {
  const block = ['files:', ...files.flatMap(({ path: file, sha256 }) => [`  - path: ${file}`, `    sha256: ${sha256}`])].join('\n') + '\n';
  const lines = text.replace(/\r\n/g, '\n').split('\n');
  const start = lines.findIndex((line) => line === 'files:');
  if (start < 0) return text.replace(/\r\n/g, '\n').replace(/\n*$/, '\n\n') + block;
  let end = start + 1;
  while (end < lines.length && (lines[end].startsWith(' ') || lines[end].startsWith('-'))) end++;
  return [...lines.slice(0, start), ...block.trimEnd().split('\n'), ...lines.slice(end)].join('\n');
}

let built = 0;
const found = new Set();
for (const folder of readdirSync(pluginsDir)) {
  const dir = path.join(pluginsDir, folder);
  const manifestPath = path.join(dir, 'plugin.yml');
  if (!existsSync(manifestPath) || (wanted.size > 0 && !wanted.has(folder))) continue;
  found.add(folder);
  if (existsSync(path.join(dir, 'tsconfig.json'))) {
    execFileSync(process.execPath, [path.join(root, 'node_modules/typescript/bin/tsc'), '-p', dir], { stdio: 'inherit' });
  }
  const text = readFileSync(manifestPath, 'utf8');
  const manifest = parse(text);
  const files = [];
  for (const entry of entriesOf(manifest)) {
    if (!entry.endsWith('.js')) throw new Error(`${folder}: ${entry} is not a .js file`);
    const srcDir = path.join(dir, 'src');
    const code = bundle(srcDir, entry.slice(0, -'.js'.length));
    const outDir = outDirOf(dir);
    mkdirSync(path.dirname(path.join(outDir, entry)), { recursive: true });
    writeFileSync(path.join(outDir, entry), code);
    files.push({ path: entry, sha256: createHash('sha256').update(code).digest('hex') });
    console.log(`${folder}/${path.relative(dir, path.join(outDir, entry)).split(path.sep).join('/')}  ${code.length} bytes`);
  }
  const updated = withFiles(text, files);
  if (updated !== text.replace(/\r\n/g, '\n')) writeFileSync(manifestPath, updated);
  built++;
}
for (const folder of wanted) {
  if (!found.has(folder)) throw new Error(`No app plugin found at ${path.relative(root, path.join(pluginsDir, folder))}`);
}
console.log(`${built} plugin(s) built.`);
