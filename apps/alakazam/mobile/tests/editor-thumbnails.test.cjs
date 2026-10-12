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
  const f = { images: [], previews: [], warnings: [], requests: [], jobs: [], active: 0, maxActive: 0, manual: false };
  const controls = ['warm', 'midnight'].map(key => ({
    key,
    kind: 'action',
    label: key,
    group: 'mood',
    live: () => [{ key }],
  }));
  const section = { key: 'moods', previewThumbnails: true, controls };
  class AbraImage {}
  function owned(effect) {
    const image = Object.assign(new AbraImage(), {
      effect,
      destroyed: 0,
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
  const source = {
    thumbnailSourceAsync(width, height) {
      f.requests.push([width, height]);
      const small = owned('base');
      small.renderThumbnailAsync = effects => {
        f.active++;
        f.maxActive = Math.max(f.maxActive, f.active);
        const effect = effects[0]?.key;
        const promise = new Promise((resolve, reject) => {
          const job = {
            effect,
            finish: () => resolve(owned(effect)),
            fail: () => reject(new Error('effect failed')),
          };
          f.jobs.push(job);
          if (!f.manual) job.finish();
        });
        return promise.finally(() => {
          f.active--;
        });
      };
      return Promise.resolve(small);
    },
    preview() {
      throw new Error('pixels must not cross the JS bridge');
    },
    rgba() {
      throw new Error('pixels must not cross the JS bridge');
    },
  };
  const bindings = { ThumbnailPreview, AbraImage };
  const nativeImages = load('native-image', { '@alakazam/mobile': bindings });
  const sections = {
    EDIT_SECTIONS: [section],
    leafControls: controls => controls,
    effectList: effect => (Array.isArray(effect) ? effect : [effect]),
    scaleBetween: () => 1,
    preloadFor: () => undefined,
    clearPreloads() {},
    applyAction: (control, image) => {
      image.effect = control.key;
    },
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
  return Object.assign(f, api, { source, section, controls, bindings, sections });
}

function build(f, previous = {}) {
  const result = { ...previous };
  const published = [];
  const job = f.buildControlThumbnails(f.source, f.section, previous, (key, preview) => {
    result[key] = preview;
    published.push(key);
  });
  return { ...job, result, published };
}

test('native thumbnails use density-adjusted dimensions without transferring pixels', async () => {
  const f = fixture();
  const job = build(f);
  assert.equal(await job.done, true);
  assert.deepEqual(f.requests, [[280, 200]]);
  assert.equal(f.THUMBNAIL_WIDTH, 112);
  assert.equal(f.THUMBNAIL_HEIGHT, 80);
  assert.equal(job.result.warm.effect, 'warm');
  assert.equal(job.result.midnight.effect, 'midnight');
  assert.equal(job.result.warm.width, 280);
  assert.equal(job.result.warm.height, 200);
  assert.ok(f.images.every(image => image.destroyed === 1));
});

test('cards publish progressively in completion order with at most two effects in flight', async () => {
  const f = fixture();
  f.manual = true;
  f.controls.push({ key: 'noir', kind: 'action', label: 'Noir', live: () => [{ key: 'noir' }] });
  const job = build(f);
  await settle();
  assert.equal(f.jobs.length, 2);
  assert.equal(job.published.length, 0);
  f.jobs[1].finish();
  await settle();
  assert.deepEqual(job.published, ['midnight']);
  assert.equal(f.jobs.length, 3);
  f.jobs[0].finish();
  f.jobs[2].finish();
  assert.equal(await job.done, true);
  assert.equal(f.maxActive, 2);
});

test('editing reuses native components', async () => {
  const f = fixture();
  f.controls[0] = { key: 'warm', kind: 'action', label: 'Rotate', live: () => [{ key: 'rotate' }] };
  const first = build(f);
  await first.done;
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
  f.manual = true;
  const second = build(f, first.result);
  await settle();
  f.jobs[2].fail();
  f.jobs[3].finish();
  assert.equal(await second.done, false);
  assert.equal(second.result.warm.updates, 1);
  assert.equal(second.result.midnight.updates, 2);
  assert.equal(f.warnings.length, 1);
  assert.ok(f.images.every(image => image.destroyed === 1));
});

test('cancellation discards late results without touching freed components or scheduling more', async () => {
  const f = fixture();
  const first = build(f);
  await first.done;
  f.manual = true;
  f.controls.push({ key: 'third', kind: 'tool', label: 'Third' });
  const second = build(f, first.result);
  await settle();
  second.cancel();
  Object.values(first.result).forEach(preview => preview.uniffiDestroy());
  f.jobs[2].finish();
  f.jobs[3].finish();
  assert.equal(await second.done, false);
  assert.equal(f.jobs.length, 4);
  assert.equal(second.published.length, 0);
  assert.equal(f.warnings.length, 0);
  assert.ok(f.images.every(image => image.destroyed === 1));
});

test('cancellation while downscaling releases its source without starting effects', async () => {
  const f = fixture();
  let resolve;
  const source = Object.assign(new f.bindings.AbraImage(), {
    destroyed: 0,
    uniffiDestroy() {
      this.destroyed++;
    },
  });
  f.source.thumbnailSourceAsync = () =>
    new Promise(done => {
      resolve = done;
    });
  const job = build(f);
  job.cancel();
  resolve(source);
  assert.equal(await job.done, false);
  assert.equal(source.destroyed, 1);
  assert.equal(f.jobs.length, 0);
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
  const small = () => ({ ...f.source, copy: small, shrunk: small, uniffiDestroy() {} });
  const base = { ...f.source, copy: small, shrunk: small };
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
      '@/src/lib/edit-sections': f.sections,
      '@/src/lib/editor-history': { recordDraftStep() {}, comparisonImage: () => null },
      '@/src/lib/editor-preloads': { preloadFor: () => undefined, clearPreloads() {} },
      '@/src/lib/editor-pictures': loadTs('lib/editor-pictures.ts', {
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
  assert.equal(f.requests.length, 0, 'main preview publishes before thumbnails start');
  assert.ok(previewSourceImage.value, 'the screen-size picture is drawn');
  assert.equal(session.currentImage.value, null, 'the full-size edit waits until after it');
  flush();
  await settle();
  assert.ok(session.currentImage.value, 'and then the full-size edit is made');
  const first = thumbnails.value;
  edits.appliedActions.next(['warm']);
  await completeReplay();
  assert.equal(thumbnails.value, first);
  assert.equal(f.previews.length, 2);
  edits.adjustments.next({ exposure: 1 });
  await completeReplay();
  assert.equal(thumbnails.value.warm, first.warm);
  assert.equal(first.warm.updates, 2);
  f.manual = true;
  edits.adjustments.next({ exposure: 2 });
  await completeReplay();
  const late = f.jobs.slice(-2);
  edits.activeSectionKey.next('tools');
  await completeReplay();
  assert.equal(Object.keys(thumbnails.value).length, 0);
  late.forEach(job => job.finish());
  await settle();
  assert.equal(Object.keys(thumbnails.value).length, 0, 'late results cannot resurrect retired cards');
  // They are freed after the frame that draws the new set, and not before it.
  await completeReplay();
  assert.ok(f.previews.every(preview => preview.destroyed === 1));
  f.manual = false;
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
    ['activeSectionKey', 'adjustments', 'appliedActions', 'focusedControlKey'].map(key => [key, subject(null)]),
  );
  let resolveDecode;
  f.bindings.AbraImage.readAsync = () =>
    new Promise(resolve => {
      resolveDecode = resolve;
    });
  const nativeImages = load('native-image', { '@alakazam/mobile': f.bindings });
  const api = load('editor-session', {
    '@alakazam/mobile': { ...f.bindings, personDetectionDownloaded: () => true },
    'react-native': { Alert: { alert() {} }, AppState: { addEventListener: () => ({ remove() {} }) } },
    '@/src/lib/batch': { batch: run => run() },
    '@/src/lib/edit-source': { resolveLocalPhotoPath: async () => 'photo.png', toLocalPath: path => path },
    '@/src/lib/editor-history': { clearTimeline() {}, keepBlemishBase() {} },
    '@/src/lib/editor-live': { startEditorLive: () => ({ unsubscribe() {} }) },
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
    copy() {
      throw new Error('a retired image must never initialize an editor');
    },
  });
  resolveDecode(late);
  await settle();
  assert.equal(late.destroyed, 1);
  assert.equal(session.originalImage.value, null);
  assert.equal(session.ready.value, false);
});
