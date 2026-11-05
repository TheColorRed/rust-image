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

const filename = path.join(__dirname, '..', 'src', 'lib', 'editor', 'preview-coordinates.ts');
const { code } = babel.transformSync(fs.readFileSync(filename, 'utf8'), {
  filename,
  babelrc: false,
  configFile: false,
  presets: ['module:@react-native/babel-preset'],
});
const loaded = { exports: {} };
vm.runInThisContext(`(function (module, exports, require) {${code}
})`)(loaded, loaded.exports, require);
const { computePreviewImagePoint, computePreviewScreenPoint } = loaded.exports;

const close = (actual, expected) => assert.ok(Math.abs(actual - expected) < 1e-6, `${actual} !== ${expected}`);

test('the middle of the photo is the middle of the box when nothing is zoomed, panned or turned', () => {
  const point = computePreviewScreenPoint(4000, 3000, { width: 400, height: 600 }, 1, { x: 0, y: 0 }, { x: 0.5, y: 0.5 });
  close(point.x, 200);
  close(point.y, 300);
});

test('a photo wider than the box is shown with bars above and below it', () => {
  // 4:3 photo in a 400 x 600 box is 400 x 300 shown, with 150 above it.
  const top = computePreviewScreenPoint(4000, 3000, { width: 400, height: 600 }, 1, { x: 0, y: 0 }, { x: 0, y: 0 });
  close(top.x, 0);
  close(top.y, 150);
});

test('it is the opposite of the screen point to a photo point, however the photo is zoomed, panned and turned', () => {
  const box = { width: 390, height: 520 };
  for (const [scale, translate, rotation] of [
    [1, { x: 0, y: 0 }, 0],
    [2.5, { x: -40, y: 25 }, 0],
    [1.3, { x: 12, y: -60 }, Math.PI / 2],
    [0.8, { x: 5, y: 5 }, 0.4],
    [3, { x: -100, y: 80 }, -1.2],
  ]) {
    for (const share of [{ x: 0.1, y: 0.2 }, { x: 0.5, y: 0.5 }, { x: 0.9, y: 0.75 }]) {
      const screen = computePreviewScreenPoint(1600, 1200, box, scale, translate, share, rotation);
      const photo = computePreviewImagePoint(1600, 1200, box, scale, translate, screen, rotation);
      assert.ok(photo, 'a point of the photo is on the photo');
      close(photo.x / 1600, share.x);
      close(photo.y / 1200, share.y);
    }
  }
});
