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
const { BehaviorSubject } = require('rxjs');

const smooth = 'action-skin-smooth';
const tan = 'action-skin-tan';
const tone = 'action-skin-tone';

function loadSource(file, dependencies) {
  const filename = path.join(__dirname, '..', 'src', file);
  const { code } = babel.transformSync(fs.readFileSync(filename, 'utf8'), {
    filename,
    babelrc: false,
    configFile: false,
    presets: ['module:@react-native/babel-preset'],
  });
  const module = { exports: {} };
  vm.runInNewContext(
    code,
    {
      module,
      exports: module.exports,
      require: name => (Object.hasOwn(dependencies, name) ? dependencies[name] : require(name)),
    },
    { filename },
  );
  return module.exports;
}

const helpers = loadSource('lib/skin-adjustments.ts', {
  '@/src/lib/edit-sections/beauty': { SKIN_SMOOTH_KEY: smooth, SKIN_TAN_KEY: tan, SKIN_TONE_KEY: tone },
});

test('Skin Tone is neutral on selection and uses the signed Beauty slider range', () => {
  const { beauty } = loadSource('lib/edit-sections/beauty.ts', {
    '@alakazam/mobile': { EffectSpec: { SkinTone: { new: fields => fields } } },
  });
  const control = beauty.controls.find(control => control.key === tone);
  assert.equal(control.min, -100);
  assert.equal(control.max, 100);
  assert.equal(control.defaultValue, 0);
  assert.equal(control.applyOnSelect, false);
  assert.equal(control.live(-40).amount, -40);
  assert.equal(control.live(65).amount, 65);
  assert.equal(helpers.adjustmentKey(tone, 1), `${tone}:person:1`);
});

test('focusing Skin Tone enables the existing person selection flow', () => {
  const { beauty } = loadSource('lib/edit-sections/beauty.ts', { '@alakazam/mobile': {} });
  const state = loadSource('state/edits.ts', {
    '@/src/lib/edit-sections': {
      SKIN_SMOOTH_KEY: smooth,
      SKIN_TAN_KEY: tan,
      SKIN_TONE_KEY: tone,
      findSection: () => beauty,
    },
  });
  const focused = [];
  const subscription = state.isSkinControlFocused$.subscribe(value => focused.push(value));
  state.focusedControlKey.next(tone);
  state.focusedControlKey.next(null);
  subscription.unsubscribe();
  assert.deepEqual(focused, [false, true, false]);
});

test('skin adjustment values are independent for each person and all people', () => {
  const values = {
    [helpers.adjustmentKey(tan, 0)]: 0.7,
    [helpers.adjustmentKey(tan, 1)]: 0.3,
    [helpers.adjustmentKey(tan, null)]: 0.1,
  };
  assert.equal(values[helpers.adjustmentKey(tan, 0)], 0.7);
  assert.equal(values[helpers.adjustmentKey(tan, 1)], 0.3);
  assert.equal(values[helpers.adjustmentKey(tan, null)], 0.1);
  assert.equal(helpers.adjustmentKey('action-brightness', 1), 'action-brightness');
  assert.deepEqual(JSON.parse(JSON.stringify(helpers.skinAdjustmentTargets(values, tan))), [
    { key: tan, personId: null, value: 0.1 },
    { key: `${tan}:person:0`, personId: 0, value: 0.7 },
    { key: `${tan}:person:1`, personId: 1, value: 0.3 },
  ]);
});

function replayFixture(selectedId) {
  const selection = { selectedId };
  const applied = [];
  const image = {
    selectedPerson: () => selection.selectedId ?? undefined,
    selectPerson: id => {
      throw new Error(`Replay must not publish a live selection change: ${id}`);
    },
    applyEffectToPerson: (effect, personId) => {
      applied.push({ ...effect, personId });
    },
  };
  const base = { copy: () => image };
  const adjustments = new BehaviorSubject({
    [`${tan}:person:0`]: 0.7,
    [`${tan}:person:1`]: 0.3,
    [`${smooth}:person:0`]: 1.5,
    [`${tone}:person:0`]: -40,
    [`${tone}:person:1`]: 65,
  });
  const sections = [
    {
      key: 'section-beauty',
      controls: [
        { key: smooth, kind: 'slider', defaultValue: 0, live: value => ({ key: smooth, value }) },
        { key: tan, kind: 'slider', defaultValue: 0, live: value => ({ key: tan, value }) },
        { key: tone, kind: 'slider', defaultValue: 0, live: value => ({ key: tone, value }) },
      ],
    },
  ];
  const replay = loadSource('lib/editor-replay.ts', {
    '@/src/lib/batch': { batch: fn => fn() },
    '@/src/lib/edit-sections': {
      EDIT_SECTIONS: sections,
      isSliderApplied: (_, value) => (value ?? 0) !== 0,
      applySlider: (control, target, value) => {
        applied.push({ key: control.key, personId: target.selectedPerson(), value });
      },
    },
    '@/src/lib/editor-history': {},
    '@/src/lib/skin-adjustments': helpers,
    '@/src/state/person-selection': { personSelection: { value: selection } },
    '@/src/lib/editor-thumbnails': {},
    '@/src/state/edits': {
      adjustments,
      appliedActions: new BehaviorSubject([]),
    },
    '@/src/state/gestures': {},
    '@/src/state/preview': {},
    '@/src/state/session': { editBaseImage: new BehaviorSubject(base) },
    '@alakazam/mobile': {},
  });
  return { replay, applied, selection };
}

test('replay applies every saved skin value without publishing intermediate selection changes', () => {
  const { replay, applied, selection } = replayFixture(2);
  replay.renderStackWithoutSlider('action-brightness');
  assert.deepEqual(applied, [
    { key: smooth, personId: 0, value: 1.5 },
    { key: tan, personId: 0, value: 0.7 },
    { key: tan, personId: 1, value: 0.3 },
    { key: tone, personId: 0, value: -40 },
    { key: tone, personId: 1, value: 65 },
  ]);
  assert.equal(selection.selectedId, 2);
});

test('live tan omits only the active person, preserving other people and smoothing', () => {
  const { replay, applied, selection } = replayFixture(1);
  assert.equal(replay.hasOtherSteps(tan), true);
  replay.renderStackWithoutSlider(tan);
  assert.deepEqual(applied, [
    { key: smooth, personId: 0, value: 1.5 },
    { key: tan, personId: 0, value: 0.7 },
    { key: tone, personId: 0, value: -40 },
    { key: tone, personId: 1, value: 65 },
  ]);
  assert.equal(selection.selectedId, 1);
});

test('choosing all people does not discard existing individual edits', () => {
  const { replay, applied, selection } = replayFixture(null);
  replay.renderStackWithoutSlider(tan);
  assert.equal(applied.filter(step => step.key === tan).length, 2);
  assert.equal(selection.selectedId, null);
});

test('live tone preserves other people and existing tan and smoothing', () => {
  const { replay, applied } = replayFixture(0);
  replay.renderStackWithoutSlider(tone);
  assert.deepEqual(applied, [
    { key: smooth, personId: 0, value: 1.5 },
    { key: tan, personId: 0, value: 0.7 },
    { key: tan, personId: 1, value: 0.3 },
    { key: tone, personId: 1, value: 65 },
  ]);
});

/** The editor's history, loaded with the real timeline logic and stand-ins for the native images and the rest of the editor. */
function loadEditorHistory({ selectedId = 0, personModel = true } = {}) {
  const timelineLib = loadSource('lib/timeline.ts', {});
  const adjustments = new BehaviorSubject({});
  const appliedActions = new BehaviorSubject([]);
  const personSelection = new BehaviorSubject({ selectedId });
  const timeline = new BehaviorSubject(timelineLib.emptyTimeline);
  const alerts = [];
  const history = loadSource('lib/editor-history.ts', {
    '@alakazam/mobile': { personDetectionDownloaded: () => personModel },
    'react-native': { Alert: { alert: (...args) => alerts.push(args) } },
    '@/src/lib/batch': { batch: fn => fn() },
    '@/src/lib/edit-sections': { EDIT_SECTIONS: [] },
    '@/src/lib/editor-replay': { renderRecipe() {} },
    '@/src/lib/error-message': { errorMessage: error => error.message },
    '@/src/lib/skin-adjustments': helpers,
    '@/src/lib/timeline': timelineLib,
    '@/src/state/person-selection': { personSelection },
    '@/src/state/edits': { adjustments, appliedActions, historyVersion: new BehaviorSubject(0) },
    '@/src/state/history': { timeline },
    '@/src/state/project-session': { historyReady: new BehaviorSubject(true) },
    '@/src/state/session': {
      currentImage: new BehaviorSubject(null),
      editBaseImage: new BehaviorSubject(null),
      originalImage: new BehaviorSubject(null),
      replayHidden: new BehaviorSubject(false),
    },
  });
  return { history, adjustments, appliedActions, personSelection, timeline, alerts };
}

test('committing and removing a skin slider change only the selected person', () => {
  const { history, adjustments, personSelection } = loadEditorHistory();
  const control = { key: tone };
  history.commitSlider(control, -40);
  personSelection.next({ selectedId: 1 });
  history.commitSlider(control, 65);
  assert.equal(adjustments.value[helpers.adjustmentKey(tone, 0)], -40);
  assert.equal(adjustments.value[helpers.adjustmentKey(tone, 1)], 65);
  history.commitSlider(control, 0);
  assert.equal(adjustments.value[helpers.adjustmentKey(tone, 1)], 0);
  history.removeSlider(control);
  assert.equal(adjustments.value[helpers.adjustmentKey(tone, 0)], -40);
  assert.equal(adjustments.value[helpers.adjustmentKey(tone, 1)], undefined);
});

/** Objects built inside the loaded modules come from another VM context, which strict deep equality rejects. */
const plain = value => JSON.parse(JSON.stringify(value));

test('every change is one step on a flat timeline, in the order it was made, and undo and redo walk it', () => {
  const { history, adjustments, appliedActions, timeline } = loadEditorHistory();
  history.commitSlider({ key: 'brightness' }, 20);
  history.toggleAction({ key: 'grayscale' });
  history.commitSlider({ key: 'contrast' }, 15);
  assert.deepEqual(plain(timeline.value.entries.map(entry => entry.change.key)), [
    'brightness',
    'grayscale',
    'contrast',
  ]);
  assert.equal(timeline.value.cursor, 3);

  history.undoHistory();
  assert.deepEqual(plain(adjustments.value), { brightness: 20 });
  assert.deepEqual(plain(appliedActions.value), ['grayscale']);
  history.undoHistory();
  assert.deepEqual(plain(appliedActions.value), []);
  history.redoHistory();
  assert.deepEqual(plain(appliedActions.value), ['grayscale']);
  assert.equal(timeline.value.entries.length, 3, 'undone steps stay for redo');

  history.commitSlider({ key: 'exposure' }, 5);
  assert.deepEqual(
    plain(timeline.value.entries.map(entry => entry.change.key)),
    ['brightness', 'grayscale', 'exposure'],
    'a new step drops the undone ones',
  );
});

test('releasing the same slider again replaces its step, and picking another option in a group replaces its step', () => {
  const { history, adjustments, timeline } = loadEditorHistory();
  history.commitSlider({ key: 'brightness' }, 10);
  history.commitSlider({ key: 'brightness' }, 25);
  history.commitSlider({ key: 'brightness' }, 15);
  assert.equal(timeline.value.entries.length, 1, 'three releases of one slider are one undo step');
  assert.equal(adjustments.value.brightness, 15);

  history.commitSlider({ key: 'contrast' }, 5);
  history.commitSlider({ key: 'brightness' }, 30);
  assert.equal(timeline.value.entries.length, 3, 'another step in between starts a new one');

  history.undoHistory();
  history.undoHistory();
  assert.equal(adjustments.value.brightness, 15);
});

test('redo is refused with an explanation when it needs the person model and it is not on the device', () => {
  const { history, adjustments, timeline, alerts } = loadEditorHistory({ personModel: false });
  history.commitSlider({ key: tone }, 40);
  history.undoHistory();
  const personKey = helpers.adjustmentKey(tone, 1);
  // A step saved with an edit aimed at person 1, after the cursor.
  timeline.next({
    entries: [{ change: { type: 'slider', key: personKey, value: 40 }, adjustments: { [personKey]: 40 }, appliedActions: [] }],
    cursor: 0,
  });
  history.redoHistory();
  assert.equal(timeline.value.cursor, 0, 'the cursor stays where it was');
  assert.equal(alerts.length, 1);
  assert.deepEqual(plain(adjustments.value), {});
});
