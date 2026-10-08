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

const filename = path.join(__dirname, '..', 'src', 'lib', 'timeline.ts');
const { code } = babel.transformSync(fs.readFileSync(filename, 'utf8'), {
  filename,
  babelrc: false,
  configFile: false,
  presets: ['module:@react-native/babel-preset'],
});
const loaded = { exports: {} };
// In this context, not a new one: objects from another context fail strict deep equality on their prototype.
vm.runInThisContext(`(function (module, exports, require) {${code}
})`)(loaded, loaded.exports, require);
const { baseIndexAt, canRedo, canUndo, emptyTimeline, moveCursor, pushEntry, recipeAt } = loaded.exports;

const slider = (key, value, adjustments) => ({
  change: { type: 'slider', key, value },
  adjustments,
  appliedActions: [],
});
const blemish = (before, x = 10) => ({
  change: { type: 'blemish', x, y: 20, radius: 5 },
  adjustments: {},
  appliedActions: [],
  before,
});

test('an empty timeline has nothing to undo or redo and an empty recipe', () => {
  assert.equal(canUndo(emptyTimeline), false);
  assert.equal(canRedo(emptyTimeline), false);
  assert.deepEqual(recipeAt(emptyTimeline.entries, 0), { adjustments: {}, appliedActions: [] });
  assert.equal(baseIndexAt(emptyTimeline.entries, 0), -1);
});

test('steps are kept in the order they were performed and the recipe is the state after the last applied one', () => {
  let timeline = emptyTimeline;
  timeline = pushEntry(timeline, slider('brightness', 20, { brightness: 20 })).timeline;
  timeline = pushEntry(timeline, slider('contrast', 15, { brightness: 20, contrast: 15 })).timeline;
  assert.deepEqual(
    timeline.entries.map(entry => entry.change.key),
    ['brightness', 'contrast'],
  );
  assert.equal(timeline.cursor, 2);
  assert.deepEqual(recipeAt(timeline.entries, timeline.cursor).adjustments, { brightness: 20, contrast: 15 });
});

test('undo and redo only move the cursor, and the steps stay for redo', () => {
  let timeline = pushEntry(emptyTimeline, slider('brightness', 20, { brightness: 20 })).timeline;
  timeline = pushEntry(timeline, slider('contrast', 15, { brightness: 20, contrast: 15 })).timeline;
  timeline = moveCursor(timeline, timeline.cursor - 1);
  assert.equal(timeline.entries.length, 2);
  assert.deepEqual(recipeAt(timeline.entries, timeline.cursor).adjustments, { brightness: 20 });
  assert.equal(canUndo(timeline), true);
  assert.equal(canRedo(timeline), true);
  timeline = moveCursor(timeline, 99);
  assert.equal(timeline.cursor, 2, 'the cursor stops at the last step');
  assert.equal(moveCursor(timeline, -5).cursor, 0, 'and at the first');
});

test('a new step after an undo drops the undone steps and reports them', () => {
  let timeline = pushEntry(emptyTimeline, slider('brightness', 20, { brightness: 20 })).timeline;
  timeline = pushEntry(timeline, slider('contrast', 15, { brightness: 20, contrast: 15 })).timeline;
  timeline = moveCursor(timeline, 1);
  const result = pushEntry(timeline, slider('exposure', 5, { brightness: 20, exposure: 5 }));
  assert.deepEqual(
    result.timeline.entries.map(entry => entry.change.key),
    ['brightness', 'exposure'],
  );
  assert.deepEqual(
    result.dropped.map(entry => entry.change.key),
    ['contrast'],
  );
  assert.equal(canRedo(result.timeline), false);
});

test('replacing the last step swaps it in place, but only at the end of the timeline', () => {
  let timeline = pushEntry(emptyTimeline, slider('brightness', 20, { brightness: 20 })).timeline;
  let result = pushEntry(timeline, slider('brightness', 30, { brightness: 30 }), true);
  assert.equal(result.timeline.entries.length, 1);
  assert.equal(result.timeline.entries[0].change.value, 30);
  assert.equal(result.dropped.length, 1);

  timeline = pushEntry(timeline, slider('contrast', 15, { brightness: 20, contrast: 15 })).timeline;
  timeline = moveCursor(timeline, 1);
  result = pushEntry(timeline, slider('exposure', 5, { brightness: 20, exposure: 5 }), true);
  assert.deepEqual(
    result.timeline.entries.map(entry => entry.change.key),
    ['brightness', 'exposure'],
    'not at the end, so it is added after the cursor like any new step',
  );
});

test('the base picture is the last applied blemish, and undoing it goes back to the one before', () => {
  let timeline = pushEntry(emptyTimeline, slider('brightness', 20, { brightness: 20 })).timeline;
  timeline = pushEntry(timeline, blemish({ adjustments: { brightness: 20 }, appliedActions: [] })).timeline;
  timeline = pushEntry(timeline, slider('contrast', 15, { contrast: 15 })).timeline;
  assert.equal(baseIndexAt(timeline.entries, 3), 1);
  assert.deepEqual(recipeAt(timeline.entries, 3).adjustments, { contrast: 15 }, 'edits after a blemish start empty');
  assert.equal(baseIndexAt(timeline.entries, 2), 1);
  assert.equal(baseIndexAt(timeline.entries, 1), -1, 'before the blemish the original is the base');
  assert.deepEqual(recipeAt(timeline.entries, 1).adjustments, { brightness: 20 });
});
