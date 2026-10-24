const assert = require('node:assert/strict');
const { test } = require('node:test');
const { loadTs } = require('./load-ts.cjs');

/**
 * A registry like the one the generated classes use, with no garbage collector: a test calls `finalize` to stand for "the wrapper
 * was collected". The generated code calls `register(ptr, p, ptr)` when it makes a handle and `unregister(ptr)` when it is destroyed.
 */
function fakeRegistry() {
  const registries = [];
  class Registry {
    constructor(cleanup) {
      this.cleanup = cleanup;
      this.registered = new Map();
      registries.push(this);
    }
    register(object, held, token) {
      this.registered.set(token ?? object, held);
    }
    unregister(token) {
      return this.registered.delete(token);
    }
    finalize(held) {
      this.cleanup(held);
    }
  }
  return { Registry, registries };
}

function setUp() {
  const { Registry, registries } = fakeRegistry();
  const target = { FinalizationRegistry: Registry };
  const warnings = [];
  const ledger = loadTs('lib/native/handle-ledger.ts', {}, { __DEV__: false, console: { warn: (...args) => warnings.push(args) } });
  ledger.installHandleLedger(target);
  // What a generated class does: one registry for the class, a pointer object per handle.
  const registry = new target.FinalizationRegistry(() => {});
  const make = pointer => {
    const ptr = { p: pointer, markDestroyed: () => undefined };
    registry.register(ptr, pointer, ptr);
    return ptr;
  };
  return { ledger, target, registry, make, warnings, Registry, registries };
}

test('a handle that is made and not released is in the ledger, and a released one is not', () => {
  const { ledger, registry, make } = setUp();
  const mark = ledger.handleLedgerMark();
  const kept = make(1n);
  const released = make(2n);
  registry.unregister(released);
  const groups = ledger.unreleasedSince(mark);
  assert.equal(groups.length, 1);
  assert.equal(groups[0].count, 1);
  registry.unregister(kept);
  assert.deepEqual(ledger.unreleasedSince(mark), []);
});

test('only the handles made after the mark are reported', () => {
  const { ledger, make } = setUp();
  make(1n);
  const mark = ledger.handleLedgerMark();
  make(2n);
  make(3n);
  assert.equal(ledger.unreleasedSince(mark)[0].count, 2);
});

test('a handle that the garbage collector frees is no longer alive, and the original cleanup still runs', () => {
  const { ledger, target, registries } = setUp();
  const cleaned = [];
  const registry = new target.FinalizationRegistry(held => cleaned.push(held));
  const ptr = { p: 9n };
  const mark = ledger.handleLedgerMark();
  registry.register(ptr, 9n, ptr);
  assert.equal(ledger.unreleasedSince(mark).length, 1);
  // The garbage collector collecting the wrapper calls the callback of the registry the ledger wrapped, which is the last one made.
  registries[registries.length - 1].finalize(9n);
  assert.deepEqual(cleaned, [9n], 'the pointer is still freed');
  assert.deepEqual(ledger.unreleasedSince(mark), [], 'and it is no longer alive');
});

test('handles are grouped by the functions that made them, the most common first', () => {
  const { ledger, registry, make } = setUp();
  const mark = ledger.handleLedgerMark();
  function renderOne() {
    return make(10n);
  }
  function renderTwo(pointer) {
    return make(pointer);
  }
  renderOne();
  renderTwo(11n);
  renderTwo(12n);
  const groups = ledger.unreleasedSince(mark);
  assert.equal(groups.length, 2);
  assert.equal(groups[0].count, 2);
  assert.match(groups[0].where, /renderTwo/);
  assert.match(groups[1].where, /renderOne/);
  void registry;
});

test('warning names how many handles are alive and where they were made, and says nothing when there are none', () => {
  const { ledger, registry, make, warnings } = setUp();
  const mark = ledger.handleLedgerMark();
  ledger.warnUnreleasedSince(mark, 'the session');
  assert.equal(warnings.length, 0);
  const ptr = make(1n);
  ledger.warnUnreleasedSince(mark, 'the session');
  assert.match(warnings[0][0], /1 native handles made before the session are still alive, from 1 places/);
  assert.equal(warnings.length, 2);
  registry.unregister(ptr);
});

test('installing twice, or with no registry, changes nothing', () => {
  const { ledger, target } = setUp();
  const first = target.FinalizationRegistry;
  ledger.installHandleLedger(target);
  assert.equal(target.FinalizationRegistry, first);
  const bare = {};
  ledger.installHandleLedger(bare);
  assert.equal('FinalizationRegistry' in bare, false);
});

test('two wrappers that hold the same pointer are two handles: releasing one leaves the other in the ledger', () => {
  const { ledger, registry, make } = setUp();
  const mark = ledger.handleLedgerMark();
  const first = make(7n);
  make(7n);
  assert.equal(ledger.unreleasedSince(mark).reduce((sum, group) => sum + group.count, 0), 2);
  registry.unregister(first);
  assert.equal(ledger.unreleasedSince(mark).reduce((sum, group) => sum + group.count, 0), 1);
});

test('a collected wrapper removes one handle of its pointer, not all of them', () => {
  const { ledger, target, registries } = setUp();
  const registry = new target.FinalizationRegistry(() => {});
  const mark = ledger.handleLedgerMark();
  const first = { p: 8n };
  const second = { p: 8n };
  registry.register(first, 8n, first);
  registry.register(second, 8n, second);
  registries[registries.length - 1].finalize(8n);
  assert.equal(ledger.unreleasedSince(mark).reduce((sum, group) => sum + group.count, 0), 1);
});
