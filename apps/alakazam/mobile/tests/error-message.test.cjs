const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');
const { test } = require('node:test');
const babel = require(
  require.resolve('@babel/core', {
    paths: [path.dirname(require.resolve('@react-native/babel-preset'))],
  }),
);

const filename = path.join(__dirname, '..', 'src', 'lib', 'error-message.ts');
const { code } = babel.transformSync(fs.readFileSync(filename, 'utf8'), {
  filename,
  babelrc: false,
  configFile: false,
  presets: ['module:@react-native/babel-preset'],
});
const moduleUnderTest = { exports: {} };
vm.runInNewContext(code, { module: moduleUnderTest, exports: moduleUnderTest.exports }, { filename });
const { errorMessage } = moduleUnderTest.exports;

test('native AI error shows its explanation rather than only its variant', () => {
  const error = new Error('AbraError.Ai');
  error.inner = Object.freeze({ message: 'Failed to load ONNX model: unsupported operator' });
  assert.equal(errorMessage(error), error.inner.message);
});

test('ordinary and non-error failures retain their existing messages', () => {
  assert.equal(errorMessage(new Error('Network unavailable')), 'Network unavailable');
  assert.equal(errorMessage('Failed'), 'Failed');
  assert.equal(errorMessage(null), 'null');
  assert.equal(errorMessage({ message: 'Fallback', inner: null }), 'Fallback');
});
