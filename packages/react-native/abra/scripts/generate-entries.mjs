// Writes the entry points of `@abra/core`: `@abra/core/adjustments`, `@abra/core/filters`, and one for each of the other files of
// the bindings crate (packages/bindings/uniffi/src).
//
// The bindings generator writes the whole library as one TypeScript module (`abra_uniffi.ts`), so each entry re-exports the part
// of it that its Rust file wrote. Which names belong to a file is read from the file: each `pub struct X`, `pub enum X` and
// top-level `pub fn some_name` gives the TypeScript names `X` (with `XLike`, `XInterface` and `X_Tags`) and `someName`.
//
// Run after the bindings are generated (the alakazam package's `build:android` does). Names that no file claims are listed and stay
// in the root entry, `@abra/core`, only.
import { mkdirSync, readFileSync, readdirSync, writeFileSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const here = path.dirname(fileURLToPath(import.meta.url));
const root = path.resolve(here, '..');
const rustDir = path.resolve(root, '../../bindings/uniffi/src');
const generated = path.resolve(root, '../alakazam/src/generated/abra_uniffi.ts');
const entriesDir = path.join(root, 'entries');
const GENERATED_FROM = '../../alakazam/src/generated/abra_uniffi';

/** Rust files that hold no exports of their own. */
const SKIP = new Set(['lib.rs', 'effect.rs']);

const camel = (name) => name.replace(/_([a-z0-9])/g, (_, letter) => letter.toUpperCase());

// Every name of the generated module, and whether it is a type only (so it must be exported with `export type`).
const source = readFileSync(generated, 'utf8');
const exported = new Map();
for (const match of source.matchAll(/^export (function|class|enum|interface|type|const|declare namespace|namespace) ([A-Za-z0-9_]+)/gm)) {
  const [, kind, name] = match;
  const typeOnly = kind === 'interface' || kind === 'type';
  exported.set(name, (exported.get(name) ?? true) && typeOnly);
}

// The names each Rust file writes.
const claimed = new Map();
const owners = new Map();
for (const file of readdirSync(rustDir).filter((name) => name.endsWith('.rs') && !SKIP.has(name)).sort()) {
  const module = file.replace(/\.rs$/, '');
  const rust = readFileSync(path.join(rustDir, file), 'utf8').split(/#\[cfg\(test\)\]/)[0];
  const names = new Set();
  for (const match of rust.matchAll(/^pub (?:struct|enum) ([A-Za-z0-9_]+)/gm)) {
    for (const suffix of ['', 'Like', 'Interface', '_Tags']) names.add(match[1] + suffix);
  }
  for (const match of rust.matchAll(/^pub fn ([a-z0-9_]+)/gm)) names.add(camel(match[1]));
  // The enums that are only declared for uniffi (`#[uniffi::remote(Enum)] pub enum X`) have no `pub` at the start of a line
  // only when indented, so they are caught above; this also keeps the order the file has.
  const found = [...names].filter((name) => exported.has(name));
  for (const name of found) {
    if (owners.has(name)) throw new Error(`${name} is written by both ${owners.get(name)}.rs and ${file}`);
    owners.set(name, module);
  }
  claimed.set(module, found.sort());
}

mkdirSync(entriesDir, { recursive: true });
const written = [];
for (const [module, names] of claimed) {
  if (names.length === 0) continue;
  const values = names.filter((name) => !exported.get(name));
  const types = names.filter((name) => exported.get(name));
  const lines = [
    `// Written by scripts/generate-entries.mjs from packages/bindings/uniffi/src/${module}.rs. Do not edit.`,
    values.length ? `export { ${values.join(', ')} } from '${GENERATED_FROM}';` : '',
    types.length ? `export type { ${types.join(', ')} } from '${GENERATED_FROM}';` : '',
  ].filter(Boolean);
  writeFileSync(path.join(entriesDir, `${module}.ts`), lines.join('\n') + '\n');
  written.push(`${module} (${names.length})`);
}

const unclaimed = [...exported.keys()].filter((name) => !owners.has(name));
console.log(`abra entries: ${written.join(', ')}`);
if (unclaimed.length) console.log(`abra entries: only in @abra/core, in no file: ${unclaimed.join(', ')}`);
