#!/usr/bin/env node
// Builds the public @abra/core type package from the app's generated UniFFI declarations.
import { execFileSync } from 'node:child_process';
import { mkdirSync, readFileSync, readdirSync, rmSync, writeFileSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../../..');
const packageDir = path.dirname(fileURLToPath(import.meta.url));
const generatedDir = path.join(root, 'packages/react-native/alakazam/src/generated');
const entriesDir = path.join(root, 'packages/react-native/abra/entries');
const dist = path.join(packageDir, 'dist');
const tsc = path.join(root, 'node_modules/typescript/bin/tsc');

rmSync(dist, { recursive: true, force: true });
mkdirSync(dist, { recursive: true });
execFileSync(
  process.execPath,
  [
    tsc,
    '--declaration', '--emitDeclarationOnly', '--skipLibCheck',
    '--target', 'ES2022', '--module', 'ESNext', '--moduleResolution', 'bundler',
    '--rootDir', generatedDir, '--outDir', dist,
    path.join(generatedDir, 'abra_uniffi.ts'),
    path.join(generatedDir, 'abra_uniffi-ffi.ts'),
  ],
  { stdio: 'inherit' },
);

const entryOut = path.join(dist, 'entries');
mkdirSync(entryOut, { recursive: true });
for (const name of readdirSync(entriesDir).filter(name => name.endsWith('.ts'))) {
  const declaration = readFileSync(path.join(entriesDir, name), 'utf8')
    .replaceAll('../../alakazam/src/generated/abra_uniffi', '../abra_uniffi')
    .replaceAll('../../alakazam/src/generated/alakazam_mobile', '../abra_uniffi');
  writeFileSync(path.join(entryOut, name.replace(/\.ts$/, '.d.ts')), declaration);
}
writeFileSync(path.join(dist, 'index.d.ts'), "export * from './abra_uniffi';\n");
console.log(`Built @abra/core declarations in ${path.relative(root, dist)}`);
