const assert = require('node:assert/strict');
const { test } = require('node:test');
const { loadTs } = require('./load-ts.cjs');

const handles = loadTs('lib/native/native-handles.ts', { react: { useEffect() {}, useRef() {}, useState() {} } }, { __DEV__: false });

/** A native handle as the generated classes make one: freed by `uniffiDestroy()`. */
const handle = () => ({
  freed: 0,
  uniffiDestroy() {
    this.freed++;
  },
});

test('building gives back the handle it made, after `finish` has changed it, and does not free it', () => {
  const made = handle();
  const result = handles.building(
    () => made,
    value => {
      value.touched = true;
    },
  );
  assert.equal(result, made);
  assert.equal(made.touched, true);
  assert.equal(made.freed, 0);
});

test('building frees the handle and lets the error go on when `finish` throws', () => {
  const made = handle();
  assert.throws(
    () =>
      handles.building(
        () => made,
        () => {
          throw new Error('a step failed');
        },
      ),
    /a step failed/,
  );
  assert.equal(made.freed, 1);
});

test('building does not free anything when `make` itself throws, because there is nothing yet', () => {
  assert.throws(
    () =>
      handles.building(
        () => {
          throw new Error('could not make it');
        },
        () => {},
      ),
    /could not make it/,
  );
});
