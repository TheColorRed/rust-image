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

/**
 * The editor's replay, loaded with its real logic and stand-ins for the native images and the rest of the editor. An image
 * records how it was made, so a test can tell the full-size photo from the screen-size copy of it.
 */
function fixture({ box = { width: 320, height: 200 } } = {}) {
  const subject = value => new BehaviorSubject(value);
  const filename = path.join(__dirname, '..', 'src', 'lib', 'editor-replay.ts');
  const { code } = babel.transformSync(fs.readFileSync(filename, 'utf8'), {
    filename,
    babelrc: false,
    configFile: false,
    presets: ['module:@react-native/babel-preset'],
  });

  const sizes = [];
  const made = [];
  /** The scale each applied control was told, in order. */
  const scales = [];
  /** The width each preload was asked for, and how many times the preloaded images were cleared. */
  const preloads = { asked: [], cleared: 0 };
  /** An image that can be shrunk and copied, and that counts how often it is destroyed. */
  const image = (name, width = 4000, height = 3000) => {
    const self = {
      name,
      width: () => width,
      height: () => height,
      destroyed: 0,
      copy: () => image(`${name} copy`, width, height),
      shrunk: (maxWidth, maxHeight) => {
        sizes.push([maxWidth, maxHeight]);
        return image(`${name} shrunk to ${maxWidth}x${maxHeight}`, maxWidth, maxHeight);
      },
      uniffiDestroy() {
        self.destroyed++;
      },
    };
    made.push(self);
    return self;
  };

  const edits = {
    activeSectionKey: subject('section'),
    adjustments: subject({}),
    appliedActions: subject([]),
    focusedControlKey: subject(null),
  };
  for (const key of Object.keys(edits)) edits[`${key}$`] = edits[key];
  const previewBoxes = subject(box);
  const previewSourceImage = subject(null);
  const session = {
    busy: subject(false),
    currentImage: subject(null),
    editBaseImage: subject(image('base')),
    originalImage: subject(image('original')),
    ready$: subject(true),
    replayHidden: subject(false),
  };
  const frames = new Map();
  const timers = new Map();
  let id = 0;
  // The screen-size pictures are a module of their own, and are loaded as they are. The preview box is in points and the
  // screen has two pixels to a point.
  const pixelsBox = subject({ width: box.width / 2, height: box.height / 2 });
  const pictures = loadTs('lib/editor-pictures.ts', {
    '@/src/state/gestures': { previewBox: pixelsBox },
    '@/src/state/preview': { previewSourceImage, showingCheckpoint: subject(false) },
    '@/src/state/session': session,
    'react-native': { Dimensions: { get: () => ({ width: 360, height: 800 }) }, PixelRatio: { get: () => 2 } },
  });
  const modules = {
    '@/src/lib/batch': { batch: run => run() },
    '@/src/lib/edit-sections': {
      EDIT_SECTIONS: [{ key: 'section', controls: [{ key: 'size', kind: 'slider', defaultValue: 0 }] }],
      applyAction() {},
      leafControls: controls => controls,
      applySlider: (_control, _image, _value, scale) => scales.push(scale),
      findSection: key => ({
        key,
        previewThumbnails: false,
        controls: [{ key: 'size', kind: 'slider', preload() {} }],
      }),
      isSliderApplied: () => true,
      scaleBetween: (image, full) => (full ? Math.min(1, image.width() / full.width()) : 1),
      sliderDefault: () => 0,
    },
    '@/src/lib/editor-preloads': {
      preloadFor: (_control, width, source) => preloads.asked.push([width, source.name]),
      clearPreloads: () => preloads.cleared++,
    },
    '@/src/lib/editor-pictures': pictures,
    '@/src/lib/editor-thumbnails': { buildControlThumbnails() {} },
    '@/src/lib/skin-adjustments': {
      isSkinAdjustment: () => false,
      adjustmentKey: key => key,
      skinAdjustmentTargets: () => [],
    },
    '@/src/state/edits': edits,
    '@/src/state/gestures': { previewBox$: previewBoxes },
    '@/src/state/person-selection': { personSelection: subject({}) },
    '@/src/state/preview': { controlThumbnails: subject({}) },
    '@/src/state/session': session,
    '@alakazam/mobile': { AbraImage: class {} },
    'react-native': { Dimensions: { get: () => ({ width: 360, height: 800 }) }, PixelRatio: { get: () => 2 } },
    rxjs: require('rxjs'),
    'rxjs/operators': require('rxjs/operators'),
  };
  const loaded = { exports: {} };
  const context = {
    __DEV__: false,
    console,
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
  };
  const globals = Object.keys(context);
  vm.runInThisContext(`(function (module, exports, require, ${globals.join(', ')}) {${code}
})`)(
    loaded,
    loaded.exports,
    specifier => {
      // Babel's own helpers are real; every other import must be one of the stand-ins.
      if (specifier.startsWith('@babel/runtime/helpers/')) return require(specifier);
      assert.ok(specifier in modules, `unexpected import: ${specifier}`);
      return modules[specifier];
    },
    ...globals.map(name => context[name]),
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
  return {
    replay: loaded.exports,
    session,
    edits,
    previewSourceImage,
    previewBoxes,
    sizes,
    made,
    scales,
    preloads,
    flush,
    pending: () => timers.size,
    pictures,
    setPixels: next => pixelsBox.next(next && { width: next.width / 2, height: next.height / 2 }),
  };
}

/** Starts the replay and runs it once: the frame, then the short delay, which replays the edit. */
const replayOnce = f => {
  const subscription = f.replay.startReplay();
  f.flush();
  return subscription;
};

test('the screen shows a copy of the photo shrunk to the preview, never the full-size photo', () => {
  const f = fixture();
  const base = f.session.editBaseImage.value;
  const subscription = replayOnce(f);
  const shown = f.previewSourceImage.value;
  assert.ok(shown, 'something is drawn');
  assert.notEqual(shown, base);
  assert.notEqual(shown, f.session.currentImage.value);
  assert.deepEqual([shown.width(), shown.height()], [320, 200], 'at the size of the preview');
  assert.deepEqual(f.sizes, [[320, 200]]);
  subscription.unsubscribe();
});

test('the full-size edit is made after the screen one, and not drawn', () => {
  const f = fixture();
  const subscription = replayOnce(f);
  const shown = f.previewSourceImage.value;
  assert.equal(f.session.currentImage.value, null, 'it has not been made yet');
  assert.equal(f.pending(), 1, 'it is waiting a moment so a run of quick edits only makes the last');
  f.flush();
  const full = f.session.currentImage.value;
  assert.ok(full, 'then it is made');
  assert.deepEqual([full.width(), full.height()], [4000, 3000], 'from the full-size photo');
  assert.equal(f.previewSourceImage.value, shown, 'and the screen still shows the screen-size picture');
  subscription.unsubscribe();
});

test('each edit is told how much smaller its photo is than the full-size one', () => {
  const f = fixture();
  const subscription = replayOnce(f);
  assert.deepEqual(f.scales, [320 / 4000], 'the screen-size edit is told the photo was shrunk to 8%');
  f.flush();
  assert.deepEqual(f.scales, [320 / 4000, 1], 'and the full-size edit is told it is the full size');
  subscription.unsubscribe();
});

test('flushing makes the waiting full-size edit at once, and does nothing when none is waiting', () => {
  const f = fixture();
  const subscription = replayOnce(f);
  f.replay.flushFullReplay();
  assert.ok(f.session.currentImage.value, 'made without waiting');
  assert.equal(f.pending(), 0, 'and no second one is left waiting');
  const made = f.session.currentImage.value;
  f.replay.flushFullReplay();
  assert.equal(f.session.currentImage.value, made, 'nothing is waiting, so nothing is made again');
  subscription.unsubscribe();
});

test('an edit replaces the picture on screen and frees the one before it, and the full-size one it replaces', () => {
  const f = fixture();
  const subscription = replayOnce(f);
  f.flush();
  const firstShown = f.previewSourceImage.value;
  const firstFull = f.session.currentImage.value;
  f.edits.adjustments.next({ exposure: 1 });
  f.flush();
  f.flush();
  assert.notEqual(f.previewSourceImage.value, firstShown);
  assert.equal(firstShown.destroyed, 1, 'the earlier screen picture is freed once');
  assert.notEqual(f.session.currentImage.value, firstFull);
  assert.equal(firstFull.destroyed, 1, 'and so is the earlier full-size one');
  subscription.unsubscribe();
});

test('the shrunk photo is made once for the same photo and size, and again when the size changes', () => {
  const f = fixture();
  const subscription = replayOnce(f);
  f.edits.adjustments.next({ exposure: 1 });
  f.flush();
  f.edits.adjustments.next({ exposure: 2 });
  f.flush();
  assert.equal(f.sizes.length, 1, 'three edits, one shrink');
  f.setPixels({ width: 300, height: 180 });
  f.previewBoxes.next({ width: 150, height: 90 });
  f.flush();
  assert.deepEqual(f.sizes.at(-1), [300, 180]);
  assert.equal(f.sizes.length, 2);
  subscription.unsubscribe();
});

test('before the preview has a size the screen is the limit', () => {
  const f = fixture();
  f.setPixels(null);
  f.pictures.showStandIn(f.session.editBaseImage.value);
  // 360 x 800 points at a density of 2.
  assert.deepEqual(f.sizes, [[720, 1600]]);
  assert.ok(f.previewSourceImage.value, 'the stand-in is on screen');
});

test('before and after shows the original shrunk to the screen too', () => {
  const f = fixture();
  const shrunk = f.pictures.comparisonImage();
  assert.deepEqual([shrunk.width(), shrunk.height()], [320, 200]);
  assert.equal(f.pictures.comparisonImage(), shrunk, 'the same copy every time');
});

test('ending the session frees every screen-size picture and cancels the waiting full-size edit', () => {
  const f = fixture();
  const subscription = replayOnce(f);
  const shown = f.previewSourceImage.value;
  const original = f.pictures.comparisonImage();
  assert.equal(f.pending(), 1);
  f.replay.releaseShownImages();
  assert.equal(f.pending(), 0, 'the full-size edit never runs');
  assert.equal(shown.destroyed, 1);
  assert.equal(original.destroyed, 1);
  assert.equal(f.pictures.shownImage(), null);
  subscription.unsubscribe();
});

test('a control is preloaded when it is clicked, from the screen-size photo, and not before; ending the session clears them', () => {
  const f = fixture();
  const subscription = replayOnce(f);
  f.flush();
  assert.deepEqual(f.preloads.asked, [], 'loading the photo and opening the tab preloads nothing');
  f.edits.focusedControlKey.next('size');
  f.flush();
  assert.deepEqual(f.preloads.asked, [[320, 'base shrunk to 320x200']]);
  assert.equal(f.preloads.cleared, 0, 'the same photo keeps them');
  f.replay.releaseShownImages();
  assert.equal(f.preloads.cleared, 1, 'the end of the session clears them');
  subscription.unsubscribe();
});
