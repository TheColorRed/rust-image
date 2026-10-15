const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');
const { test } = require('node:test');
const { loadTs } = require('./load-ts.cjs');
const babel = require(
  require.resolve('@babel/core', {
    paths: [path.dirname(require.resolve('@react-native/babel-preset'))],
  }),
);

globalThis.__DEV__ = false;
const filename = path.join(__dirname, '..', 'src', 'lib', 'edit-sections.ts');
const { code } = babel.transformSync(fs.readFileSync(filename, 'utf8'), {
  filename,
  babelrc: false,
  configFile: false,
  presets: ['module:@react-native/babel-preset'],
});
const loaded = { exports: {} };
// The sections and the native packages are not needed to read a control's settings, so they load as empty modules.
// The cache of what a control preloads is a module of its own, and is loaded as it is.
const analysisState = { analysisVersion: { value: 0, next(value) { this.value = value; } } };
const preloads = loadTs('lib/editor/editor-preloads.ts', { '@/src/lib/editor/editor-pictures': { fitted: source => Object.assign(source.clone(), { width: () => source.width() }) }, '@abra/core/mask': { TransformAlgorithm: {} }, '@/src/state/analysis': analysisState });
const stubbed = specifier =>
  specifier === '@/src/lib/editor/editor-preloads'
    ? preloads
    : specifier.startsWith('.') || specifier.startsWith('@')
      ? {}
      : require(specifier);
vm.runInThisContext(`(function (module, exports, require) {${code}
})`)(loaded, loaded.exports, stubbed);
const {
  applyAction,
  applySlider,
  isMultiSlider,
  isSliderApplied,
  sliderDefault,
  sliderSpecs,
  sliderValues,
  scaleBetween,
} = loaded.exports;
const { preloadFor, releasePreloads } = preloads;

const single = extra => ({ kind: 'slider', key: 'single', label: 'Single', ...extra });
const multi = extra => ({ kind: 'slider', key: 'multi', label: 'Multi', sliderLabel: ['A', 'B'], ...extra });

test('a control with one slider has the defaults for what it leaves out', () => {
  assert.deepEqual(sliderSpecs(single({})), [{ label: 'Single', min: 0, max: 100, step: 1, defaultValue: 0 }]);
  assert.equal(isMultiSlider(single({})), false);
  assert.equal(sliderDefault(single({ defaultValue: 128 })), 128);
});

test('each item of an array is a slider, and a setting given once is the same for all of them', () => {
  const control = multi({ min: [-50, -10], max: 100, step: [1, 5], defaultValue: [0, 20] });
  assert.deepEqual(sliderSpecs(control), [
    { label: 'A', min: -50, max: 100, step: 1, defaultValue: 0 },
    { label: 'B', min: -10, max: 100, step: 5, defaultValue: 20 },
  ]);
  assert.equal(isMultiSlider(control), true);
  assert.deepEqual(sliderDefault(control), [0, 20]);
});

test('an item left out of an array, or undefined, takes the default', () => {
  // eslint-disable-next-line no-sparse-arrays
  const holes = multi({ min: [, 1], max: [undefined, 50] });
  assert.deepEqual(
    sliderSpecs(holes).map(spec => [spec.min, spec.max]),
    [
      [0, 100],
      [1, 50],
    ],
  );
});

test('arrays of different lengths are an error that names the control', () => {
  assert.throws(() => sliderSpecs(multi({ min: [0, 1, 2] })), /"multi".*same length/);
  assert.throws(() => sliderSpecs(multi({ defaultValue: [0] })), /same length/);
});

test('a one-item array is still a control with several sliders, stored as an array', () => {
  const control = single({ sliderLabel: ['Only'] });
  assert.equal(isMultiSlider(control), true);
  assert.deepEqual(sliderDefault(control), [0]);
});

test('the values of the sliders come from what is committed and from the defaults where there is nothing', () => {
  const control = multi({ defaultValue: [10, 20] });
  assert.deepEqual(sliderValues(control, undefined), [10, 20]);
  assert.deepEqual(sliderValues(control, [1, 2]), [1, 2]);
  assert.deepEqual(sliderValues(control, [7]), [7, 20], 'a short array is filled in');
  assert.deepEqual(sliderValues(single({ defaultValue: 5 }), 9), [9]);
});

test('a control that applies on select is applied once it has a value', () => {
  assert.equal(isSliderApplied(multi({}), undefined), false);
  assert.equal(isSliderApplied(multi({}), [0, 0]), true);
  assert.equal(isSliderApplied(single({}), 0), true);
});

test('a control whose default means no change is applied only when a slider is off its default', () => {
  const control = multi({ applyOnSelect: false, defaultValue: [0, 50] });
  assert.equal(isSliderApplied(control, undefined), false);
  assert.equal(isSliderApplied(control, [0, 50]), false);
  assert.equal(isSliderApplied(control, [0, 51]), true);
  assert.equal(isSliderApplied(control, [-1, 50]), true);
  // The default is 0 when it is left out, so a slider at 0 is not applied.
  assert.equal(isSliderApplied(single({ applyOnSelect: false }), 0), false);
  assert.equal(isSliderApplied(single({ applyOnSelect: false }), 3), true);
});

test('a slider with no label of its own takes the label of the control', () => {
  assert.deepEqual(
    sliderSpecs(single({})).map(spec => spec.label),
    ['Single'],
  );
  // eslint-disable-next-line no-sparse-arrays
  assert.deepEqual(
    sliderSpecs(multi({ sliderLabel: [, 'B'] })).map(spec => spec.label),
    ['Multi', 'B'],
  );
  assert.deepEqual(
    sliderSpecs(multi({ sliderLabel: undefined, min: [0, 1] })).map(spec => spec.label),
    ['Multi', 'Multi'],
  );
});

/** An image that records what is done to it. */
const fakeImage = () => {
  const log = { replaced: [], destroyed: [] };
  const photo = { name: 'photo', uniffiDestroy: () => log.destroyed.push('photo') };
  const image = { height: () => 3000, clone: () => photo, replaceWith: made => log.replaced.push(made.name) };
  return { image, photo, log };
};

test('a tool gets the value of every slider, and the changed photo is the result when it returns nothing', () => {
  const { image, log } = fakeImage();
  const seen = [];
  const control = multi({ tool: (photo, values) => void seen.push([photo.name, values]) });
  applySlider(control, image, [3, 4]);
  assert.deepEqual(seen, [['photo', [3, 4]]]);
  assert.deepEqual(log.replaced, ['photo']);
  assert.deepEqual(log.destroyed, ['photo']);
});

test('a tool that returns an image has that image used, and both are freed', () => {
  const { image, log } = fakeImage();
  const returned = { name: 'returned', uniffiDestroy: () => log.destroyed.push('returned') };
  applySlider(multi({ tool: () => returned }), image, [1, 2]);
  assert.deepEqual(log.replaced, ['returned']);
  assert.deepEqual(log.destroyed.sort(), ['photo', 'returned']);
});

test('a control with one slider still gets its number as a one-item array, and a short value is filled with the default', () => {
  const seen = [];
  const { image } = fakeImage();
  applySlider(single({ tool: (_photo, values) => void seen.push(values) }), image, 42);
  applySlider(multi({ defaultValue: [1, 2], tool: (_photo, values) => void seen.push(values) }), image, [9]);
  assert.deepEqual(seen, [[42], [9, 2]]);
});

const wide = width => ({ width: () => width });

test('the scale is how much smaller an image is than the full-size photo, and never more than 1', () => {
  assert.equal(scaleBetween(wide(1000), wide(4000)), 0.25);
  assert.equal(scaleBetween(wide(4000), wide(4000)), 1, 'the full-size photo');
  assert.equal(scaleBetween(wide(900), wide(900)), 1, 'a photo that already fits the screen is not shrunk');
  assert.equal(scaleBetween(wide(5000), wide(4000)), 1, 'never more than 1');
  assert.equal(scaleBetween(wide(1000), null), 1, 'with no full-size photo to compare, nothing was shrunk');
  assert.equal(scaleBetween(wide(1000), wide(0)), 1, 'and an empty one is not divided by');
});

test('a tool, an apply and a thumb are told the scale, which is 1 unless the editor says otherwise', () => {
  const told = [];
  const { image } = fakeImage();
  const record =
    name =>
    (_photo, ...rest) => {
      told.push([name, { scale: rest.at(-1).scale }]);
    };
  applySlider(single({ tool: record('tool') }), image, 5, 0.25);
  applySlider(
    single({
      apply: (_image, _value, context) => {
        told.push(['apply', { scale: context.scale }]);
      },
    }),
    image,
    5,
    0.5,
  );
  applyAction({ kind: 'action', key: 'a', label: 'A', tool: record('action tool') }, image, 0.125);
  applySlider(single({ tool: record('default') }), image, 5);
  assert.deepEqual(told, [
    ['tool', { scale: 0.25 }],
    ['apply', { scale: 0.5 }],
    ['action tool', { scale: 0.125 }],
    ['default', { scale: 1 }],
  ]);
});

test('a preload runs once for a photo width, its result reaches the tool, and clearing frees it', () => {
  releasePreloads();
  const mask = {
    destroyed: 0,
    uniffiDestroy() {
      this.destroyed++;
    },
  };
  let runs = 0;
  const seen = [];
  const control = single({
    preload: () => (runs++, mask),
    tool: (_photo, _values, context) => void seen.push(context.preloaded),
  });
  const { image, log } = fakeImage();
  image.width = () => 320;
  assert.equal(preloadFor(control, 320), undefined, 'without a source, nothing is made');
  assert.deepEqual(preloadFor(control, 320, image), [mask], 'one image becomes an array of one');
  applySlider(control, image, 1);
  applySlider(control, image, 2);
  assert.equal(runs, 1, 'made once, then reused');
  assert.deepEqual(seen, [[mask], [mask]]);
  assert.deepEqual(log.destroyed.slice(0, 1), ['photo'], 'the copy it was made from is freed');
  image.width = () => 4000;
  applySlider(control, image, 3);
  assert.equal(runs, 2, 'a different size makes its own');
  releasePreloads();
  assert.equal(mask.destroyed, 2);
  assert.equal(preloadFor(control, 320), undefined, 'and nothing is left');
});

test('a preload that returns a promise runs in the background, once, and the editor is told when it is made', async () => {
  releasePreloads();
  const mask = { destroyed: 0, width: () => 320, uniffiDestroy() { this.destroyed++; } };
  let runs = 0;
  let finish;
  const control = single({ preload: () => (runs++, new Promise(resolve => (finish = resolve))) });
  const { image } = fakeImage();
  image.width = () => 320;
  const before = analysisState.analysisVersion.value;
  assert.equal(preloadFor(control, 320, image), undefined, 'nothing to use yet, and nothing waits for it');
  assert.equal(preloadFor(control, 320, image), undefined, 'asking again meanwhile');
  assert.equal(runs, 1, 'does not start it again while it runs');
  finish({ a: mask });
  await new Promise(resolve => setTimeout(resolve, 0));
  assert.equal(analysisState.analysisVersion.value, before + 1, 'the editor is told to draw again');
  assert.deepEqual(preloadFor(control, 320, image), { a: mask }, 'and then the tool gets it');
  assert.equal(runs, 1, 'without running again');
  releasePreloads();
  assert.equal(mask.destroyed, 1, 'and it is freed with the photo');
});

test('a background preload that could not run is not kept as an answer, and one that finishes after its photo is gone is freed', async () => {
  releasePreloads();
  const { image } = fakeImage();
  image.width = () => 320;
  let runs = 0;
  const missing = single({ preload: () => (runs++, Promise.resolve(undefined)) });
  preloadFor(missing, 320, image);
  await new Promise(resolve => setTimeout(resolve, 0));
  assert.equal(preloadFor(missing, 320, image), undefined);
  assert.equal(runs, 2, 'nothing was kept, so it is tried again');

  const mask = { destroyed: 0, uniffiDestroy() { this.destroyed++; } };
  let finish;
  const late = single({ preload: () => new Promise(resolve => (finish = resolve)) });
  preloadFor(late, 320, image);
  releasePreloads();
  finish({ a: mask });
  await new Promise(resolve => setTimeout(resolve, 0));
  assert.equal(mask.destroyed, 1, 'what it made is freed');
  assert.equal(preloadFor(late, 320), undefined, 'and not kept for the next photo');
});

test('controls sharing a preload function share one result at the same photo width', () => {
  releasePreloads();
  const mask = {
    destroyed: 0,
    uniffiDestroy() {
      this.destroyed++;
    },
  };
  let runs = 0;
  const shared = () => (runs++, mask);
  const first = single({ key: 'first', preload: shared });
  const second = single({ key: 'second', preload: shared });
  const { image } = fakeImage();
  image.width = () => 320;
  assert.deepEqual(preloadFor(first, 320, image), [mask]);
  assert.deepEqual(preloadFor(second, 320, image), [mask]);
  assert.equal(runs, 1);
  releasePreloads();
  assert.equal(mask.destroyed, 1, 'the shared result is owned and freed once');
});

test('a control with no preload is told no preloaded images', () => {
  const seen = [];
  const { image } = fakeImage();
  applySlider(single({ tool: (_photo, _values, context) => void seen.push(context.preloaded) }), image, 1);
  assert.deepEqual(seen, [undefined]);
});

test('a preload can return an object of masks, an array, one mask or nothing, and all of them are freed', () => {
  releasePreloads();
  const data = () => ({
    destroyed: 0,
    uniffiDestroy() {
      this.destroyed++;
    },
  });
  const [a, b, c, d] = [data(), data(), data(), data()];
  const shapes = { object: { a, b }, array: [c], single: d, nothing: undefined };
  const seen = {};
  for (const [name, made] of Object.entries(shapes)) {
    const control = single({
      key: name,
      preload: () => made,
      tool: (_p, _v, context) => void (seen[name] = context.preloaded),
    });
    const { image } = fakeImage();
    image.width = () => 100;
    applySlider(control, image, 1);
  }
  assert.deepEqual(seen.object, { a, b }, 'an object stays an object');
  assert.deepEqual(seen.array, [c]);
  assert.deepEqual(seen.single, [d], 'one mask becomes an array of one');
  assert.deepEqual(seen.nothing, [], 'nothing becomes an empty array');
  releasePreloads();
  assert.deepEqual(
    [a, b, c, d].map(item => item.destroyed),
    [1, 1, 1, 1],
  );
});

test('what a preload made is made again when the control gets a new preload function, and the old is freed', () => {
  releasePreloads();
  const data = () => ({
    destroyed: 0,
    uniffiDestroy() {
      this.destroyed++;
    },
  });
  const [old, fresh] = [data(), data()];
  const { image } = fakeImage();
  image.width = () => 100;
  assert.equal(preloadFor(single({ preload: () => old }), 100, image)[0], old);
  assert.equal(
    preloadFor(single({ preload: () => fresh }), 100)?.[0],
    undefined,
    'without a source, the old is not handed out',
  );
  assert.equal(old.destroyed, 1, 'it was made by a function the control no longer has');
  assert.equal(preloadFor(single({ preload: () => fresh }), 100, image)[0], fresh);
  releasePreloads();
});
