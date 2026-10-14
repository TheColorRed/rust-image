const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');
const { test } = require('node:test');
const { loadTs } = require('./load-ts.cjs');
const { BehaviorSubject } = require('rxjs');
const babel = require(
  require.resolve('@babel/core', {
    paths: [path.dirname(require.resolve('@react-native/babel-preset'))],
  }),
);

function load(name, modules, globals = {}) {
  const filename = path.join(__dirname, '..', 'src', 'lib', `${name}.ts`);
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
      __DEV__: false,
      require: key => {
        if (key.startsWith('@babel/runtime/helpers/')) return require(key);
        assert.ok(key in modules, `unexpected import: ${key}`);
        return modules[key];
      },
      ...globals,
    },
    { filename },
  );
  return module.exports;
}

const settle = () => new Promise(resolve => setImmediate(resolve));

function fixture() {
  const f = { images: [], previews: [], warnings: [] };
  const controls = ['warm', 'midnight'].map(key => ({
    key,
    kind: 'action',
    label: key,
    group: 'mood',
    tool: photo => {
      photo.effect = key;
    },
  }));
  const section = { key: 'moods', previewThumbnails: true, controls };
  class AbraImage {}
  function owned(effect) {
    const image = Object.assign(new AbraImage(), {
      effect,
      destroyed: 0,
      width: () => 16,
      height: () => 16,
      clone: () => owned(image.effect),
      replaceWith(other) {
        image.effect = other.effect;
      },
      uniffiDestroy() {
        assert.equal(++this.destroyed, 1);
      },
    });
    f.images.push(image);
    return image;
  }
  class ThumbnailPreview {
    constructor(image, label, width, height) {
      Object.assign(this, { label, width, height, updates: 0, destroyed: 0 });
      this.setImage(image);
      f.previews.push(this);
    }
    setImage(image) {
      assert.equal(this.destroyed, 0);
      this.effect = image.effect;
      this.updates++;
    }
    uniffiDestroy() {
      assert.equal(++this.destroyed, 1);
    }
  }
  const bindings = { ThumbnailPreview, AbraImage };
  const nativeImages = load('native-image', { '@alakazam/mobile': bindings });
  const sections = {
    EDIT_SECTIONS: [section],
    leafControls: controls => controls,
    scaleBetween: () => 1,
    preloadFor: () => undefined,
    clearPreloads() {},
    findSection: key => (key === 'moods' ? section : { key, previewThumbnails: false, controls: [] }),
  };
  const api = load(
    'editor-thumbnails',
    {
      '@alakazam/mobile': bindings,
      'react-native': { PixelRatio: { get: () => 2.5 } },
      '@/src/lib/edit-sections': sections,
      '@/src/lib/native-image': nativeImages,
    },
    { console: { warn: (...args) => f.warnings.push(args) } },
  );
  const fresh = () => owned('base');
  return Object.assign(f, api, { fresh, section, controls, bindings, sections });
}

function build(f, previous = {}) {
  const result = { ...previous };
  const published = [];
  const job = f.buildControlThumbnails(f.fresh(), f.section, previous, (key, preview) => {
    result[key] = preview;
    published.push(key);
  });
  return { ...job, result, published };
}

test('native thumbnails use density-adjusted dimensions without transferring pixels', async () => {
  const f = fixture();
  const job = build(f);
  assert.equal(await job.done, true);
  assert.equal(f.THUMBNAIL_WIDTH, 112);
  assert.equal(f.THUMBNAIL_HEIGHT, 80);
  assert.equal(job.result.warm.effect, 'warm');
  assert.equal(job.result.midnight.effect, 'midnight');
  assert.equal(job.result.warm.width, 280);
  assert.equal(job.result.warm.height, 200);
  assert.ok(f.images.every(image => image.destroyed === 1));
});

test('editing reuses native components', async () => {
  const f = fixture();
  const first = build(f);
  await first.done;
  f.controls[0].tool = photo => {
    photo.effect = 'rotate';
  };
  const second = build(f, first.result);
  await second.done;
  assert.equal(second.result.warm, first.result.warm);
  assert.equal(second.result.warm.effect, 'rotate');
  assert.equal(f.previews.length, 2);
  assert.equal(first.result.warm.updates, 2);
  assert.ok(f.images.every(image => image.destroyed === 1));
});

test('failed effects warn, keep their previous card and do not block siblings', async () => {
  const f = fixture();
  const first = build(f);
  await first.done;
  f.controls[0].tool = () => {
    throw new Error('effect failed');
  };
  const second = build(f, first.result);
  assert.equal(await second.done, false);
  assert.equal(second.result.warm.updates, 1);
  assert.equal(second.result.midnight.updates, 2);
  assert.equal(f.warnings.length, 1);
  assert.ok(f.images.every(image => image.destroyed === 1));
});

test('replay defers thumbnails, caches exclusive groups and discards retired builds', async () => {
  const f = fixture();
  const subject = value => new BehaviorSubject(value);
  const edits = {
    activeSectionKey: subject('moods'),
    adjustments: subject({}),
    appliedActions: subject([]),
    focusedControlKey: subject(null),
  };
  for (const key of Object.keys(edits)) edits[`${key}$`] = edits[key];
  const thumbnails = subject({});
  const previewSourceImage = subject(null);
  const size = { width: () => 16, height: () => 16 };
  const base = f.fresh();
  const session = {
    busy: subject(false),
    currentImage: subject(null),
    editBaseImage: subject(base),
    ready$: subject(true),
    replayHidden: subject(false),
  };
  const frames = new Map();
  const timers = new Map();
  let id = 0;
  const replay = load(
    'editor-replay',
    {
      '@alakazam/mobile': f.bindings,
      rxjs: require('rxjs'),
      'rxjs/operators': require('rxjs/operators'),
      '@/src/lib/batch': { batch: run => run() },
      '@/src/lib/edit-sections': {
        ...f.sections,
        applyAction: (control, image) => control.tool(image),
        applySlider() {},
        isSliderApplied: () => true,
        layerSheet: () => undefined,
        sliderContext: () => ({}),
        sliderDefault: () => 0,
        sliderValues: () => ({}),
      },
      '@/src/lib/editor-history': { recordDraftStep() {}, comparisonImage: () => null },
      '@/src/lib/editor-preloads': { preloadFor: () => undefined, clearPreloads() {} },
      '@/src/lib/analysis': { personIn: () => undefined },
      '@/src/lib/layers': { withLayers: photo => photo },
      '@/src/lib/editor-pictures': loadTs('lib/editor-pictures.ts', {
        '@abra/core/canvas': { ResizeTarget: { Fit: { new: size => size } } },
        '@abra/core/transform': { resize: () => ({ apply() {} }) },
        '@/src/state/gestures': { previewBox: subject({ width: 320, height: 200 }) },
        '@/src/state/preview': { previewSourceImage, showingCheckpoint: subject(false) },
        '@/src/state/session': session,
        'react-native': { Dimensions: { get: () => ({ width: 320, height: 640 }) }, PixelRatio: { get: () => 1 } },
      }),
      '@/src/lib/skin-adjustments': {},
      '@/src/state/person-selection': { personSelection: subject(null) },
      '@/src/lib/editor-thumbnails': f,
      '@/src/state/edits': edits,
      '@/src/state/gestures': { previewBox$: subject({ width: 320, height: 200 }) },
      '@/src/state/preview': {
        controlThumbnails: thumbnails,
        previewSourceImage,
        showingCheckpoint: subject(false),
      },
      '@/src/state/session': session,
    },
    {
      requestAnimationFrame: run => {
        frames.set(++id, run);
        return id;
      },
      cancelAnimationFrame: key => frames.delete(key),
      setTimeout: run => {
        timers.set(++id, run);
        return id;
      },
      clearTimeout: key => timers.delete(key),
    },
  );

  const flush = () => {
    for (const [key, run] of [...frames]) {
      frames.delete(key);
      run();
    }
    for (const [key, run] of [...timers]) {
      timers.delete(key);
      run();
    }
  };
  const completeReplay = async () => {
    flush();
    flush();
    await settle();
  };
  const subscription = replay.startReplay();
  flush();
  assert.equal(f.previews.length, 0, 'main preview publishes before thumbnails start');
  assert.ok(previewSourceImage.value, 'the picture is drawn');
  assert.ok(session.currentImage.value, 'with nothing applied the edit is the full-size photo, which costs nothing to make');
  flush();
  await settle();
  const first = thumbnails.value;
  edits.appliedActions.next(['warm']);
  await completeReplay();
  assert.equal(thumbnails.value, first);
  assert.equal(f.previews.length, 2);
  edits.adjustments.next({ exposure: 1 });
  await completeReplay();
  assert.equal(thumbnails.value.warm, first.warm);
  assert.equal(first.warm.updates, 2);
  edits.activeSectionKey.next('tools');
  await completeReplay();
  assert.equal(Object.keys(thumbnails.value).length, 0);
  // They are freed after the frame that draws the new set, and not before it.
  await completeReplay();
  assert.ok(f.previews.every(preview => preview.destroyed === 1));
  edits.activeSectionKey.next('moods');
  await completeReplay();
  assert.notEqual(thumbnails.value.warm, first.warm);
  replay.clearControlThumbnails();
  edits.adjustments.next({});
  await completeReplay();
  assert.equal(Object.keys(thumbnails.value).length, 2, 'reset invalidates unchanged cache inputs');
  subscription.unsubscribe();
  replay.clearControlThumbnails();
  await completeReplay();
  assert.ok(f.previews.every(preview => preview.destroyed === 1));
});

test('closing an editor during async decoding releases late native images without publishing them', async () => {
  const f = fixture();
  const subject = value => new BehaviorSubject(value);
  const session = Object.fromEntries(
    ['busy', 'currentImage', 'editBaseImage', 'loadError', 'originalImage', 'ready'].map(key => [key, subject(null)]),
  );
  const edits = Object.fromEntries(
    ['activeSectionKey', 'adjustments', 'appliedActions', 'focusedControlKey', 'openGroupKey'].map(key => [key, subject(null)]),
  );
  let resolveDecode;
  const readImageAsync = () =>
    new Promise(resolve => {
      resolveDecode = resolve;
    });
  const nativeImages = load('native-image', { '@alakazam/mobile': f.bindings });
  const api = load('editor-session', {
    '@alakazam/mobile': { ...f.bindings, personDetectionDownloaded: () => true, readImageAsync },
    'react-native': { Alert: { alert() {} }, AppState: { addEventListener: () => ({ remove() {} }) } },
    '@/src/lib/batch': { batch: run => run() },
    '@abra/core/mask': { TransformAlgorithm: {} },
    '@/src/state/analysis': { people: subject(null), skinMask: subject(null) },
    '@/src/lib/analysis': { personIn: () => undefined, clearAnalysis() {} },
    '@/src/lib/eye-makeup': { makeupMasks: () => undefined },
    '@/src/lib/editor-preloads': { releasePreloads() {}, clearPreloads() {} },
    '@/src/lib/edit-source': { resolveLocalPhotoPath: async () => 'photo.png', toLocalPath: path => path },
    '@/src/lib/editor-history': { clearTimeline() {}, keepBlemishBase() {} },
    '@/src/lib/editor-preview': { startEditorPreview: () => ({ unsubscribe() {} }) },
    '@/src/lib/editor-replay': {
      startReplay: () => ({ unsubscribe() {} }),
      clearControlThumbnails() {},
      renderRecipe() {},
      releaseShownImages() {},
    },
    '@/src/lib/editor-pictures': { showStandIn() {} },
    '@/src/lib/error-message': { errorMessage: String },
    '@/src/lib/native-image': nativeImages,
    '@/src/lib/project-store': { projectImageUri: () => '' },
    '@/src/lib/timeline': { recipeAt: () => ({ adjustments: {}, appliedActions: [] }) },
    '@/src/state/edits': edits,
    '@/src/state/history': { timeline: subject(null) },
    '@/src/state/preview': { previewSourceImage: subject(null), showingCheckpoint: subject(false) },
    '@/src/state/person-selection': { personSelection: subject(null) },
    '@/src/state/project-session': { historyReady: subject(false), openProject: subject(null) },
    '@/src/state/projects': { updateProject: async () => {} },
    '@/src/state/session': session,
  });
  const close = api.openEditorSession({});
  await settle();
  close();
  const late = Object.assign(new f.bindings.AbraImage(), {
    destroyed: 0,
    uniffiDestroy() {
      this.destroyed++;
    },
    clone() {
      throw new Error('a retired image must never initialize an editor');
    },
  });
  resolveDecode(late);
  await settle();
  assert.equal(late.destroyed, 1);
  assert.equal(session.originalImage.value, null);
  assert.equal(session.ready.value, false);
});
