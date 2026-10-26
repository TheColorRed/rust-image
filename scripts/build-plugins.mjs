#!/usr/bin/env node
// Repository convenience wrapper around the npm-published @alakazam/plugin CLI.
import { execFileSync } from 'node:child_process';
import path from 'node:path';
import { existsSync, readdirSync } from 'node:fs';
import { fileURLToPath, pathToFileURL } from 'node:url';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const pluginsDir = path.join(root, 'apps/alakazam/mobile/plugins');
const tool = path.join(root, 'packages/plugins/plugin-toolkit/build-plugins.mjs');
const requested = process.argv.slice(2);
const args = requested.length ? requested.map(value => {
  const direct = path.resolve(value);
  return path.isAbsolute(value) || value.includes('/') || value.includes('\\') ? direct : path.join(pluginsDir, value);
}) : readdirSync(pluginsDir).map(folder => path.join(pluginsDir, folder)).filter(dir => existsSync(path.join(dir, 'plugin.yml')));
const tsc = path.join(root, 'node_modules/typescript/bin/tsc');
for (const dir of args) {
  if (existsSync(path.join(dir, 'tsconfig.json'))) execFileSync(process.execPath, [tsc, '-p', dir], { stdio: 'inherit' });
}
process.argv = [process.argv[0], tool, ...args];
await import(pathToFileURL(tool).href);
