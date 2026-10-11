const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');
const babel = require(
  require.resolve('@babel/core', {
    paths: [path.dirname(require.resolve('@react-native/babel-preset'))],
  }),
);

/**
 * Loads one TypeScript file of `src` with its real code and stand-ins for everything it imports: an import that is not in
 * `modules` fails the test, so a new import has to be dealt with. `globals` are the names the code reads without importing.
 */
function loadTs(file, modules, globals = {}) {
  const filename = path.join(__dirname, '..', 'src', file);
  const { code } = babel.transformSync(fs.readFileSync(filename, 'utf8'), {
    filename,
    babelrc: false,
    configFile: false,
    presets: ['module:@react-native/babel-preset'],
  });
  const loaded = { exports: {} };
  const names = Object.keys(globals);
  vm.runInThisContext(`(function (module, exports, require, ${names.join(', ')}) {${code}
})`)(
    loaded,
    loaded.exports,
    specifier => {
      // Babel's own helpers are real; every other import must be one of the stand-ins.
      if (specifier.startsWith('@babel/runtime/helpers/')) return require(specifier);
      assert.ok(specifier in modules, `${file} imports ${specifier}, which has no stand-in`);
      return modules[specifier];
    },
    ...names.map(name => globals[name]),
  );
  return loaded.exports;
}

module.exports = { loadTs };
