const assert = require('node:assert/strict');
const { test } = require('node:test');
const { loadTs } = require('./load-ts.cjs');

// The file draws with native objects; what is tested here is only the numbers it saves, so those imports are empty stand-ins.
const nothing = {};
// A stand-in for the library's color finder: it hands back the colors it was set up to find. Each is a color with the few things the code
// asks of one: its red, green and blue, its hue, saturation and value, and its complement (here the opposite red, green and blue).
let found = [];
// The color scheme the code asked the library for last.
let asked;
const color = rgb => {
  const [red, green, blue] = rgb;
  const high = Math.max(red, green, blue);
  const low = Math.min(red, green, blue);
  return {
    rgb: () => new Uint8Array(rgb).buffer,
    hsv: () => [0, high === 0 ? 0 : (high - low) / high, high / 255],
    // Two colors, the accent and its opposite, or five for the scheme that gives five, each a little different from the one before.
    harmony: scheme => {
      asked = scheme;
      const count = scheme === 'split-complementary' ? 5 : 2;
      return Array.from({ length: count }, (_, index) => color(index === 0 ? rgb : rgb.map(channel => (channel + index * 10) % 256)));
    },
  };
};
const quantize = () => ({
  withTolerance() {
    return { palette: () => found.map(color) };
  },
});
const byPlace = stops => [...stops].sort((a, b) => a.at - b.at);
const {
  GRADIENT_AREA,
  GRADIENT_AT,
  GRADIENT_CUSTOM,
  GRADIENT_HANDLES,
  GRADIENT_LOOKS,
  GRADIENT_STRENGTH,
  PHOTO_HARMONIES,
  decodeStops,
  encodeStops,
  gradientFromPhoto,
  gradientShown,
  gradientValues,
  lookAsGradient,
  withCustomGradient,
} = loadTs('effects/gradient-overlay.ts', {
  '@abra/core/adjustments': { quantize },
  '@abra/core/blend': { BlendMode: new Proxy({}, { get: (_, name) => name }) },
  '@abra/core/color': { Harmony: { Complementary: { new: () => 'complementary' }, Analogous: { new: () => 'analogous' }, SplitComplementary: { new: () => 'split-complementary' }, Compound: { new: () => 'compound' }, Shades: { new: count => `shades ${count}` }, Triadic: { new: () => 'triadic' }, Square: { new: () => 'square' }, Monochromatic: { new: count => `monochromatic ${count}` } }, ColorSpace: { Oklab: 'oklab' }, HueDirection: { Shorter: 'shorter' } },
  '@abra/core/gradient': nothing,
  '@abra/core/image': nothing,
  '@/src/lib/editor/editor-preloads': { derivedFor: () => undefined },
  '@/src/lib/color/gradient-math': { stopsOf: value => byPlace(value.colors) },
  '@/src/lib/native/native-handles': { owning: work => work(x => x) },
});

const made = {
  colors: [
    { at: 0, color: [10, 20, 30] },
    { at: 0.4, color: [200, 100, 50] },
    { at: 1, color: [255, 255, 255] },
  ],
  alphas: [
    { at: 0, alpha: 0 },
    { at: 1, alpha: 255 },
  ],
};

test('a gradient the person made survives being saved and read back', () => {
  assert.deepEqual(decodeStops(withCustomGradient([], made)), made);
});

test('nothing saved gives the starting settings, so a gradient never moved still has handles', () => {
  const all = gradientValues([]);
  assert.equal(all[GRADIENT_AT.strength], GRADIENT_STRENGTH);
  assert.equal(all[GRADIENT_AT.look], 0);
  assert.deepEqual(all.slice(GRADIENT_AT.startX, GRADIENT_AT.startX + GRADIENT_HANDLES.length), [...GRADIENT_HANDLES]);
  assert.equal(all.length, GRADIENT_AT.stops);
});

test('saving a gradient keeps the strength, the blend and the handles, and sets the look to custom', () => {
  const before = [0.5, 3, 2, 0.1, 0.2, 0.8, 0.9];
  const saved = withCustomGradient(before, made);
  // The area was never chosen, so it is the whole photo.
  assert.deepEqual(saved.slice(0, GRADIENT_AT.stops), [0.5, GRADIENT_CUSTOM, 2, 0.1, 0.2, 0.8, 0.9, GRADIENT_AREA.whole]);
  // Making another replaces the first and does not pile up behind it.
  const again = withCustomGradient(saved, { colors: [{ at: 0, color: [1, 2, 3] }], alphas: [{ at: 0, alpha: 255 }] });
  assert.equal(again.length, GRADIENT_AT.stops + encodeStops({ colors: [{ at: 0, color: [1, 2, 3] }], alphas: [{ at: 0, alpha: 255 }] }).length);
});

test('what is saved that is not a gradient is not read as one', () => {
  assert.equal(decodeStops([1, 0, 0, 0.5, 0.15, 0.5, 0.85]), undefined);
  // Cut short: it says three color stops but the numbers stop.
  assert.equal(decodeStops([1, GRADIENT_CUSTOM, 0, 0.5, 0.15, 0.5, 0.85, 3, 0, 1, 2, 3]), undefined);
});

test('the gradient shown is the ready-made look chosen, or the one the person made', () => {
  assert.deepEqual(gradientShown([1, 1]), lookAsGradient(1));
  assert.deepEqual(gradientShown(withCustomGradient([], made)), made);
  // A custom look whose stops are gone falls back to the first look and not to nothing.
  assert.deepEqual(gradientShown([1, GRADIENT_CUSTOM]), lookAsGradient(0));
});

test('every ready-made look is a gradient the editor can start from, with a place and a strength for each stop', () => {
  GRADIENT_LOOKS.forEach((look, index) => {
    const gradient = lookAsGradient(index);
    assert.equal(gradient.colors.length, look.stops.length);
    assert.equal(gradient.alphas.length, look.stops.length);
    assert.deepEqual(decodeStops(withCustomGradient([], gradient)), gradient);
  });
});

test('the gradient of a photo goes from its most colorful color to the color opposite it, not from its biggest color', () => {
  // The sand is most of a beach photo, but the swimsuit is what stands out.
  found = [[202, 175, 144], [111, 139, 165], [207, 121, 91], [80, 60, 53]];
  const gradient = gradientFromPhoto({});
  assert.deepEqual(gradient.colors, [
    { at: 0, color: [207, 121, 91] },
    // The fake library's second color: the accent a little shifted.
    { at: 1, color: [217, 131, 101] },
  ]);
  assert.deepEqual(gradient.alphas, [{ at: 0, alpha: 255 }, { at: 1, alpha: 255 }]);
});

test('a gray photo still gives a gradient, and a photo with no colors found gives none', () => {
  found = [[120, 120, 120]];
  assert.equal(gradientFromPhoto({}).colors.length, 2);
  found = [];
  assert.equal(gradientFromPhoto({}), undefined);
});

test('the scheme chosen is the one the library is asked for, and the opposite color is the one it starts with', () => {
  found = [[207, 121, 91]];
  gradientFromPhoto({});
  assert.equal(asked, 'complementary');
  for (const [scheme, wanted] of [['neighbors', 'analogous'], ['three-way', 'triadic'], ['four-way', 'square'], ['one-color', 'monochromatic 5']]) {
    gradientFromPhoto({}, scheme);
    assert.equal(asked, wanted, scheme);
  }
});

test('every color scheme of the library can be chosen, and the one that comes in theme order is put in the order of a gradient', () => {
  found = [[100, 50, 20]];
  for (const [scheme, wanted] of [['soft-opposite', 'split-complementary'], ['neighbor-and-opposite', 'compound'], ['darker', 'shades 5']]) {
    gradientFromPhoto({}, scheme);
    assert.equal(asked, wanted, scheme);
  }
  // The five colors come as the accent, then 10, 20, 30 and 40 more; in gradient order they are the 4th, the accent, the 3rd, 2nd and 5th.
  const gradient = gradientFromPhoto({}, 'soft-opposite');
  assert.deepEqual(gradient.colors.map(stop => stop.color[0]), [130, 100, 120, 110, 140]);
  assert.deepEqual(gradient.colors.map(stop => stop.at), [0, 0.25, 0.5, 0.75, 1]);
  // Seven schemes are named, and there is no scheme named twice.
  assert.equal(new Set(PHOTO_HARMONIES.map(each => each.key)).size, PHOTO_HARMONIES.length);
  assert.equal(PHOTO_HARMONIES.length, 8);
});

test('the area is whole photo until chosen, and saving a gradient keeps the area that was chosen', () => {
  assert.equal(gradientValues([])[GRADIENT_AT.area], GRADIENT_AREA.whole);
  const before = [0.5, 3, 2, 0.1, 0.2, 0.8, 0.9, GRADIENT_AREA.background];
  assert.equal(withCustomGradient(before, { colors: [{ at: 0, color: [1, 2, 3] }], alphas: [{ at: 0, alpha: 255 }] })[GRADIENT_AT.area], GRADIENT_AREA.background);
  // The stops start after the area, so a saved gradient is read back whole whichever area it is for.
  const saved = withCustomGradient(before, { colors: [{ at: 0, color: [1, 2, 3] }, { at: 1, color: [4, 5, 6] }], alphas: [{ at: 0, alpha: 255 }] });
  assert.deepEqual(decodeStops(saved).colors, [{ at: 0, color: [1, 2, 3] }, { at: 1, color: [4, 5, 6] }]);
});
