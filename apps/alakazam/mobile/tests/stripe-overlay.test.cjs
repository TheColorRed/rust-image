const assert = require('node:assert/strict');
const { test } = require('node:test');
const { loadTs } = require('./load-ts.cjs');

// The file draws with native objects; what is tested here is only the numbers it saves, so those imports are empty stand-ins.
const nothing = {};
const { STRIPE_AT, STRIPE_DEFAULTS, STRIPE_LOOKS, STRIPE_WORDS, decodeWords, encodeWords, stripeValues, withWords } = loadTs('effects/stripe-overlay.ts', {
  '@abra/core/blend': { BlendMode: new Proxy({}, { get: (_, name) => name }) },
  '@abra/core/color': nothing,
  '@abra/core/image': nothing,
  '@abra/core/sprites': nothing,
  '@/src/lib/fonts/text-image': { installedFamilies: () => [], renderText: () => null },
  '@/src/lib/native/native-handles': { owning: work => work(x => x) },
});

test('stripes that were never changed have the starting values and the ready-made words', () => {
  assert.deepEqual(stripeValues([]), [STRIPE_DEFAULTS.strength, STRIPE_DEFAULTS.look, STRIPE_DEFAULTS.showSub, STRIPE_DEFAULTS.capsTitle, STRIPE_DEFAULTS.capsSub]);
  assert.deepEqual(decodeWords([]), { ...STRIPE_WORDS, family: '' });
});

test('words come back as they were saved, including empty ones and letters outside the basic set', () => {
  const words = { title: 'Brooklyn', subtitle: '', family: 'Édito Sans' };
  assert.deepEqual(decodeWords([...stripeValues([]), ...encodeWords(words)]), words);
  assert.deepEqual(decodeWords(withWords([0.5, 2, 0, 1, 0], words)), words);
});

test('saving other words keeps the sliders and the colors', () => {
  const saved = withWords([0.5, 2, 0, 1, 0], { title: 'A', subtitle: 'B', family: 'F' });
  assert.deepEqual(saved.slice(0, STRIPE_AT.words), [0.5, 2, 0, 1, 0]);
  assert.deepEqual(withWords(saved, { title: 'C', subtitle: '', family: '' }).slice(0, STRIPE_AT.words), [0.5, 2, 0, 1, 0]);
});

test('every ready-made look has a name and colors', () => {
  for (const look of STRIPE_LOOKS) {
    assert.ok(look.label);
    for (const rgb of [look.stripe, look.under, look.ink, look.underInk]) assert.equal(rgb.length, 3);
  }
});

test('values saved under an older layout show the ready-made text and not the saved numbers read as letters', () => {
  // Five sliders, the look, three switches and the words, as the control saved them before the sub shift slider was added.
  const old = [1, -8, 0.5, 0.5, 0, 0, 1, 1, 1, 1, 1, 1, 1, ...encodeWords({ title: 'Brooklyn', subtitle: 'Nine-Nine', family: 'Archivo' })];
  assert.deepEqual(decodeWords(old), { ...STRIPE_WORDS, family: '' });
  // Text that is cut short, or that has a length that runs past the end, is not read either.
  assert.deepEqual(decodeWords([...stripeValues([]), 5, 65, 66]), { ...STRIPE_WORDS, family: '' });
});
