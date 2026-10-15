const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const { test } = require('node:test');

/**
 * Saving a control or a tool re-runs it and every module that imports it, and a re-run module starts again empty. The state of
 * the editor, and the modules that hold native images, must not be among them, so they may only import types from the controls
 * and from the replay.
 */
const src = path.join(__dirname, '..', 'src');
const reRun = [
  /^@\/src\/lib\/edit-sections$/,
  /^@\/src\/lib\/edit-sections\/(?!keys$)/,
  /^@\/src\/lib\/editor\/editor-(replay|history|session|live)$/,
  /^@\/src\/effects\//,
];
const stable = [
  ...fs.readdirSync(path.join(src, 'state')).map(name => `state/${name}`),
  'lib/editor/editor-pictures.ts',
  'lib/editor/editor-preloads.ts',
  'lib/edit-sections/keys.ts',
];

test('the editor state and the modules that hold native images do not import the controls or the replay', () => {
  for (const file of stable) {
    const code = fs.readFileSync(path.join(src, file), 'utf8');
    for (const match of code.matchAll(/^import\s+(type\s+)?([^;]*?)\s+from\s+'([^']+)';/gms)) {
      const [, typeOnly, names, specifier] = match;
      // `import { type A }` is erased too, when every name is a type.
      const onlyTypes =
        typeOnly ||
        (/^\{[^}]*\}$/.test(names) &&
          names
            .replace(/[{}]/g, '')
            .split(',')
            .every(name => !name.trim() || name.trim().startsWith('type ')));
      if (onlyTypes) continue;
      assert.ok(
        !reRun.some(pattern => pattern.test(specifier)),
        `${file} imports ${specifier}, which is re-run when a control is saved`,
      );
    }
  }
});
