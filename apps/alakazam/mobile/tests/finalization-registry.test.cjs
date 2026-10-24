const assert = require('node:assert/strict');
const { test } = require('node:test');
const { loadTs } = require('./load-ts.cjs');

/** A `WeakRef` a test can empty by hand, standing for "the garbage collector collected the object". */
function fakeEngine() {
  const references = [];
  class WeakRef {
    constructor(object) {
      this.object = object;
      references.push(this);
    }
    deref() {
      return this.object;
    }
  }
  const timers = [];
  const setTimeout = (callback, ms) => {
    timers.push({ callback, ms });
    return timers.length;
  };
  return { WeakRef, references, timers, setTimeout };
}

function setUp(engine = fakeEngine(), extra = {}) {
  const target = { WeakRef: engine.WeakRef, ...extra };
  const warnings = [];
  const loaded = loadTs(
    'lib/native/finalization-registry.ts',
    {},
    { setTimeout: engine.setTimeout, console: { warn: (...args) => warnings.push(args) }, WeakMap, Set, globalThis: {} },
  );
  const installed = loaded.installFinalizationRegistry(target);
  return { engine, target, installed, warnings, loaded };
}

test('a handle whose wrapper was collected is freed once, and one whose wrapper is alive is not', () => {
  const { engine, target, installed } = setUp();
  const freed = [];
  const registry = new target.FinalizationRegistry(held => freed.push(held));
  const alive = { p: 1n };
  const gone = { p: 2n };
  registry.register(alive, 1n, alive);
  registry.register(gone, 2n, gone);
  installed.sweep();
  assert.deepEqual(freed, [], 'nothing is collected yet');
  engine.references[1].object = undefined;
  installed.sweep();
  assert.deepEqual(freed, [2n]);
  installed.sweep();
  assert.deepEqual(freed, [2n], 'a handle is freed only once');
});

test('a handle that was unregistered is not freed when its wrapper is collected', () => {
  const { engine, target, installed } = setUp();
  const freed = [];
  const registry = new target.FinalizationRegistry(held => freed.push(held));
  const wrapper = { p: 3n };
  registry.register(wrapper, 3n, wrapper);
  assert.equal(registry.unregister(wrapper), true);
  assert.equal(registry.unregister(wrapper), false, 'a second unregister finds nothing');
  engine.references[0].object = undefined;
  installed.sweep();
  assert.deepEqual(freed, []);
});

test('the check runs again while handles are registered, and stops when there are none', () => {
  const { engine, target, installed } = setUp();
  const registry = new target.FinalizationRegistry(() => {});
  const wrapper = { p: 4n };
  registry.register(wrapper, 4n, wrapper);
  assert.equal(engine.timers.length, 1, 'the first registration starts the checks');
  const before = engine.timers.length;
  installed.sweep();
  assert.equal(engine.timers.length, before + 1, 'a handle is still waiting, so there is another check');
  engine.references[0].object = undefined;
  installed.sweep();
  const after = engine.timers.length;
  installed.sweep();
  assert.equal(engine.timers.length, after, 'nothing is registered any more, so nothing is scheduled');
});

test('one handle that cannot be freed does not stop the others, and says so', () => {
  const { engine, target, installed, warnings } = setUp();
  const freed = [];
  const registry = new target.FinalizationRegistry(held => {
    if (held === 5n) throw new Error('already freed');
    freed.push(held);
  });
  const first = { p: 5n };
  const second = { p: 6n };
  registry.register(first, 5n, first);
  registry.register(second, 6n, second);
  engine.references[0].object = undefined;
  engine.references[1].object = undefined;
  installed.sweep();
  assert.deepEqual(freed, [6n]);
  assert.equal(warnings.length, 1);
});

test('it does nothing when the engine has a registry already, or has no WeakRef', () => {
  const engine = fakeEngine();
  const own = class {};
  const withRegistry = setUp(engine, { FinalizationRegistry: own });
  assert.equal(withRegistry.installed, undefined);
  assert.equal(withRegistry.target.FinalizationRegistry, own);

  const noWeakRef = { };
  const loaded = loadTs('lib/native/finalization-registry.ts', {}, { setTimeout: engine.setTimeout, console, WeakMap, Set, globalThis: {} });
  assert.equal(loaded.installFinalizationRegistry(noWeakRef), undefined);
  assert.equal('FinalizationRegistry' in noWeakRef, false);
});
