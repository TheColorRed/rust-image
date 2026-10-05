const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');
const { test } = require('node:test');
const { BehaviorSubject } = require('rxjs');
const babel = require(
  require.resolve('@babel/core', {
    paths: [path.dirname(require.resolve('@react-native/babel-preset'))],
  }),
);

function fixture() {
  const filename = path.join(__dirname, '..', 'src', 'lib', 'preview-handoff.ts');
  const { code } = babel.transformSync(fs.readFileSync(filename, 'utf8'), {
    filename,
    babelrc: false,
    configFile: false,
    presets: ['module:@react-native/babel-preset'],
  });
  let now = 0;
  let next = 0;
  let frames = new Map();
  const module = { exports: {} };
  vm.runInNewContext(
    code,
    {
      module,
      exports: module.exports,
      Date: { now: () => now },
      requestAnimationFrame: callback => {
        frames.set(++next, callback);
        return next;
      },
      cancelAnimationFrame: id => frames.delete(id),
    },
    { filename },
  );
  return {
    wait: module.exports.waitForPreviewFrame,
    tick(time) {
      now = time;
      const pending = frames;
      frames = new Map();
      for (const callback of pending.values()) callback();
    },
  };
}

test('handoff keeps the live effect until a completed native frame is presented', () => {
  const { wait, tick } = fixture();
  let presented = false;
  let hidden = false;
  wait(
    { hasFrame: () => presented },
    () => (hidden = true),
    error => assert.fail(error.message),
  );
  tick(150);
  tick(600);
  assert.equal(hidden, false);
  presented = true;
  tick(616);
  assert.equal(hidden, true);
});

test('cancelling a retired preview stops native queries and handoff', () => {
  const { wait, tick } = fixture();
  let destroyed = false;
  const cancel = wait(
    {
      hasFrame: () => {
        assert.equal(destroyed, false);
        return false;
      },
    },
    () => assert.fail('retired preview completed'),
    error => assert.fail(error.message),
  );
  tick(16);
  cancel();
  destroyed = true;
  tick(32);
});

test('a stalled presentation reports an error instead of hiding the live effect', () => {
  const { wait, tick } = fixture();
  const errors = [];
  wait(
    { hasFrame: () => false },
    () => assert.fail('unpresented frame completed'),
    error => errors.push(error.message),
  );
  tick(5000);
  tick(6000);
  assert.deepEqual(errors, ['The edited preview did not reach the display.']);
});

test('a newer live renderer interaction is not hidden by the previous committed frame', () => {
  const filename = path.join(__dirname, '..', 'src', 'lib', 'live-renderer.ts');
  const { code } = babel.transformSync(fs.readFileSync(filename, 'utf8'), {
    filename,
    babelrc: false,
    configFile: false,
    presets: ['module:@react-native/babel-preset'],
  });
  const module = { exports: {} };
  const bindings = {
    ImagePreview: class {
      send() {}
      uniffiDestroy() {}
    },
    Message: { SliderMove: { new: (key, value) => ({ key, value }) } },
  };
  vm.runInNewContext(
    code,
    {
      module,
      exports: module.exports,
      require: name => (name === '@alakazam/mobile' ? bindings : require(name)),
    },
    { filename },
  );
  const live = new module.exports.LiveRenderer(new BehaviorSubject(null));
  assert.equal(
    live.ensure('base', () => ({ width: () => 4, height: () => 4 })),
    true,
  );
  live.show();
  const { wait, tick } = fixture();
  let presented = false;
  const committedActivity = live.activity;
  wait(
    { hasFrame: () => presented },
    () => {
      if (live.activity === committedActivity) live.hide();
    },
    error => assert.fail(error.message),
  );
  live.slide('action-brightness', 30);
  assert.ok(live.activity > committedActivity);
  presented = true;
  tick(16);
  assert.ok(live.surface.value);
  live.release();
});
