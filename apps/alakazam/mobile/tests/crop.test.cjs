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

const filename = path.join(__dirname, '..', 'src', 'lib', 'tools', 'crop.ts');
const { code } = babel.transformSync(fs.readFileSync(filename, 'utf8'), {
  filename,
  babelrc: false,
  configFile: false,
  presets: ['module:@react-native/babel-preset'],
});
const loaded = { exports: {} };
vm.runInThisContext(`(function (module, exports, require) {${code}
})`)(loaded, loaded.exports, require);
const {
  FULL_CROP,
  MIN_CROP_SIDE,
  containCropRect,
  coverCropFrame,
  cropPixels,
  cropToScreen,
  fitCropInTurn,
  fitCropToAspect,
  moveCropArea,
  moveCropCorner,
  panToCentreAboveBar,
  panToKeepPoint,
  photoBoundsOnScreen,
  cropZoomFloor,
  screenToCrop,
  slideFrameInto,
  turnDegrees,
  turnedPhoto,
  zoomOutToFrame,
  zoomToFrame,
} = loaded.exports;

const close = (actual, expected) => assert.ok(Math.abs(actual - expected) < 1e-9, `${actual} !== ${expected}`);

test('a full crop keeps the whole photo', () => {
  assert.deepEqual(cropPixels(FULL_CROP, 400, 300), { x: 0, y: 0, width: 400, height: 300 });
});

test('a crop is never outside the photo or empty', () => {
  const pixels = cropPixels({ left: 0.9999, top: 0.9999, right: 0.9999, bottom: 0.9999 }, 400, 300);
  assert.ok(pixels.width >= 1 && pixels.height >= 1);
  assert.ok(pixels.x + pixels.width <= 400 && pixels.y + pixels.height <= 300);
});

test('a 16:9 crop of a 4:3 photo spans the width and is centered vertically', () => {
  const photo = { width: 4000, height: 3000 };
  const rect = fitCropToAspect(FULL_CROP, 16 / 9, photo);
  close(rect.left, 0);
  close(rect.right, 1);
  close(rect.top, 0.125);
  close(rect.bottom, 0.875);
});

test('a freeform aspect leaves the crop alone', () => {
  const rect = { left: 0.1, top: 0.2, right: 0.6, bottom: 0.9 };
  assert.deepEqual(fitCropToAspect(rect, null, { width: 400, height: 300 }), rect);
});

test('a turn makes the picture as big as its bounding box', () => {
  const photo = { width: 400, height: 300 };
  assert.deepEqual(turnedPhoto(photo, 0), photo);
  const quarter = turnedPhoto(photo, Math.PI / 2);
  close(quarter.width, 300);
  close(quarter.height, 400);
  const tilted = turnedPhoto(photo, Math.PI / 6);
  close(tilted.width, 400 * Math.cos(Math.PI / 6) + 300 * Math.sin(Math.PI / 6));
});

test('a turn wraps into the range Abra rotates by', () => {
  close(turnDegrees(Math.PI / 2), 90);
  close(turnDegrees(-Math.PI / 2), -90);
  close(turnDegrees(Math.PI * 2 + 0.1), 5.729577951308232);
});

test('a full crop is still valid after a quarter turn', () => {
  const photo = { width: 400, height: 300 };
  assert.deepEqual(fitCropInTurn(FULL_CROP, photo, Math.PI / 2), FULL_CROP);
});

test('a full crop shrinks when the photo is tilted, and then holds still', () => {
  const photo = { width: 400, height: 300 };
  const tilted = fitCropInTurn(FULL_CROP, photo, Math.PI / 18);
  assert.ok(tilted.right - tilted.left < 1);
  assert.ok(tilted.bottom - tilted.top < 1);
  const again = fitCropInTurn(tilted, photo, Math.PI / 18);
  close(again.left, tilted.left);
  close(again.top, tilted.top);
  close(again.right, tilted.right);
  close(again.bottom, tilted.bottom);
});

test('a crop on the screen maps back to the same part of a turned photo', () => {
  const photo = { width: 400, height: 300 };
  const box = { width: 320, height: 480 };
  const turn = Math.PI / 9;
  const rect = { left: 0.2, top: 0.3, right: 0.7, bottom: 0.8 };
  const frame = cropToScreen(rect, photo, box, 1.6, { x: 12, y: -30 }, turn);
  const back = screenToCrop(frame, photo, box, 1.6, { x: 12, y: -30 }, turn);
  close(back.left, rect.left);
  close(back.top, rect.top);
  close(back.right, rect.right);
  close(back.bottom, rect.bottom);
});

test('dragging a corner with an aspect keeps the crop square', () => {
  const photo = { width: 400, height: 300 };
  const rect = moveCropCorner(FULL_CROP, 'top-left', -50, 0, photo, 1);
  close(rect.left, 0.25);
  close(rect.top, 0);
  close(rect.right, 1);
  close(rect.bottom, 1);
});

test('dragging a corner outward stops at the photo edge', () => {
  const photo = { width: 400, height: 300 };
  const rect = moveCropCorner(
    { left: 0.25, top: 0.25, right: 0.75, bottom: 0.75 },
    'bottom-right',
    1000,
    1000,
    photo,
    null,
  );
  close(rect.right, 1);
  close(rect.bottom, 1);
});

test('dragging a corner past the opposite corner keeps the minimum size', () => {
  const photo = { width: 400, height: 300 };
  const rect = moveCropCorner(
    { left: 0.25, top: 0.25, right: 0.75, bottom: 0.75 },
    'top-left',
    1000,
    1000,
    photo,
    null,
  );
  close(rect.right - rect.left, MIN_CROP_SIDE);
  close(rect.bottom - rect.top, MIN_CROP_SIDE);
  close(rect.right, 0.75);
  close(rect.bottom, 0.75);
});

test('moving the crop area slides it and stops at the photo edges', () => {
  const photo = { width: 100, height: 100 };
  const rect = moveCropArea({ left: 0.5, top: 0.5, right: 0.75, bottom: 0.75 }, 100, -100, photo);
  close(rect.left, 0.75);
  close(rect.right, 1);
  close(rect.top, 0);
  close(rect.bottom, 0.25);
});

test('moving the crop area stops at the visible bounds, not the photo edge', () => {
  const photo = { width: 100, height: 100 };
  const bounds = { left: 0, top: 0.2, right: 1, bottom: 1 };
  const rect = moveCropArea({ left: 0.5, top: 0.5, right: 0.75, bottom: 0.75 }, 0, -100, photo, bounds);
  close(rect.top, 0.2);
  close(rect.bottom, 0.45);
});

test('dragging a corner stops at the visible bounds', () => {
  const photo = { width: 400, height: 300 };
  const bounds = { left: 0, top: 0.2, right: 1, bottom: 1 };
  const rect = moveCropCorner(
    { left: 0.25, top: 0.5, right: 0.75, bottom: 0.75 },
    'top-left',
    0,
    -1000,
    photo,
    null,
    bounds,
  );
  close(rect.top, 0.2);
  close(rect.right, 0.75);
  close(rect.bottom, 0.75);
});

test('a corner dragged with an aspect stays inside the visible bounds', () => {
  const photo = { width: 400, height: 300 };
  const bounds = { left: 0, top: 0.2, right: 1, bottom: 1 };
  const rect = moveCropCorner(FULL_CROP, 'top-left', -1000, -1000, photo, 1, bounds);
  assert.ok(rect.top >= 0.2 - 1e-9 && rect.left >= -1e-9);
  close((rect.right - rect.left) * photo.width, (rect.bottom - rect.top) * photo.height);
});

test('a crop fitted to an aspect is the largest that fits the visible bounds', () => {
  const photo = { width: 400, height: 300 };
  const bounds = { left: 0, top: 0.2, right: 1, bottom: 1 };
  const rect = fitCropToAspect(FULL_CROP, 1, photo, bounds);
  close(rect.top, 0.2);
  close(rect.bottom, 1);
  close(rect.left, 0.2);
  close(rect.right, 0.8);
});

test('a crop larger than the visible bounds shrinks about its middle to fit them', () => {
  const photo = { width: 400, height: 300 };
  const bounds = { left: 0, top: 0.2, right: 1, bottom: 1 };
  const rect = containCropRect(FULL_CROP, photo, bounds);
  close(rect.left, 0.1);
  close(rect.right, 0.9);
  close(rect.top, 0.2);
  close(rect.bottom, 1);
});

test('a crop outside the visible bounds slides back inside them', () => {
  const photo = { width: 400, height: 300 };
  const bounds = { left: 0.5, top: 0.5, right: 1, bottom: 1 };
  const rect = containCropRect({ left: 0, top: 0, right: 0.2, bottom: 0.2 }, photo, bounds);
  close(rect.left, 0.5);
  close(rect.right, 0.7);
  close(rect.top, 0.5);
  close(rect.bottom, 0.7);
});

test('a full crop at 1x fills the photo as it is fitted in the preview', () => {
  const photo = { width: 400, height: 300 };
  const box = { width: 200, height: 300 };
  const frame = cropToScreen(FULL_CROP, photo, box, 1, { x: 0, y: 0 });
  assert.deepEqual(frame, { left: 0, top: 75, right: 200, bottom: 225 });
});

test('a crop on screen maps back to the same crop in the photo at any zoom and pan', () => {
  const photo = { width: 400, height: 300 };
  const box = { width: 200, height: 300 };
  const rect = { left: 0.2, top: 0.3, right: 0.7, bottom: 0.9 };
  for (const [scale, translate] of [
    [1, { x: 0, y: 0 }],
    [2.5, { x: 30, y: -40 }],
    [0.8, { x: -12, y: 5 }],
  ]) {
    const back = screenToCrop(cropToScreen(rect, photo, box, scale, translate), photo, box, scale, translate);
    close(back.left, rect.left);
    close(back.top, rect.top);
    close(back.right, rect.right);
    close(back.bottom, rect.bottom);
  }
});

test('zooming the photo under a fixed frame changes the crop, not the frame', () => {
  const photo = { width: 400, height: 300 };
  const box = { width: 200, height: 300 };
  const frame = cropToScreen({ left: 0.25, top: 0.25, right: 0.75, bottom: 0.75 }, photo, box, 1, { x: 0, y: 0 });
  const zoomed = screenToCrop(frame, photo, box, 2, { x: 0, y: 0 });
  close(zoomed.left, 0.375);
  close(zoomed.right, 0.625);
});

test('the photo is zoomed to cover a frame that would show past its edge', () => {
  const photo = { width: 400, height: 300 };
  const box = { width: 200, height: 300 };
  const frame = { left: 0, top: 75, right: 200, bottom: 225 };
  const limited = coverCropFrame(frame, photo, box, 1, { x: 0, y: 0 });
  close(limited.scale, 1);
  close(limited.x, 0);
  close(limited.y, 0);
  const pannedOff = coverCropFrame(frame, photo, box, 1, { x: 80, y: -80 });
  close(pannedOff.x, 0);
  close(pannedOff.y, 0);
});

test('a frame larger than the photo at this zoom raises the zoom to cover it', () => {
  const photo = { width: 400, height: 300 };
  const box = { width: 200, height: 300 };
  const frame = { left: 0, top: 0, right: 200, bottom: 300 };
  const covered = coverCropFrame(frame, photo, box, 0.5, { x: 0, y: 0 });
  close(covered.scale, 2);
});

test('zooming to a crop that is centred in the box only raises the zoom', () => {
  const box = { width: 200, height: 300 };
  const frame = { left: 50, top: 100, right: 150, bottom: 200 };
  const zoom = zoomToFrame(frame, box, 1, { x: 0, y: 0 }, 0, 0.9, 100);
  close(zoom.scale, 1.8);
  close(zoom.x, 0);
  close(zoom.y, 0);
});

test('zooming to an off-centre crop pans the photo so the crop lands in the middle of the box', () => {
  const box = { width: 200, height: 300 };
  const frame = { left: 100, top: 100, right: 200, bottom: 200 };
  const zoom = zoomToFrame(frame, box, 1, { x: 0, y: 0 }, 0, 0.9, 100);
  close(zoom.scale, 1.8);
  close(zoom.x, -90);
  close(zoom.y, 0);
});

test('zooming to a crop follows the preview turn', () => {
  const box = { width: 200, height: 300 };
  const frame = { left: 50, top: 150, right: 150, bottom: 250 };
  const zoom = zoomToFrame(frame, box, 1, { x: 0, y: 0 }, Math.PI / 2, 0.9, 100);
  close(zoom.scale, 1.8);
  close(zoom.x, -90);
  close(zoom.y, 0);
});

test('zooming to a crop never zooms out, and stops at the maximum zoom', () => {
  const box = { width: 200, height: 300 };
  const filling = { left: 0, top: 0, right: 200, bottom: 300 };
  assert.equal(zoomToFrame(filling, box, 1, { x: 0, y: 0 }, 0, 0.9, 100), null);
  const tiny = { left: 95, top: 145, right: 105, bottom: 155 };
  const capped = zoomToFrame(tiny, box, 1, { x: 0, y: 0 }, 0, 0.9, 2);
  close(capped.scale, 2);
});

test('zooming to a crop centres it in a smaller area above the gestures row', () => {
  const box = { width: 200, height: 300 };
  const area = { left: 20, top: 20, right: 180, bottom: 180 };
  const frame = { left: 75, top: 125, right: 125, bottom: 175 };
  const zoom = zoomToFrame(frame, box, 1, { x: 0, y: 0 }, 0, 0.9, 100, area);
  close(zoom.scale, 2.88);
  close(zoom.x, 0);
  close(zoom.y, -50);
});

test('zooming to a crop in a smaller area follows the preview turn', () => {
  const box = { width: 200, height: 300 };
  const area = { left: 20, top: 20, right: 180, bottom: 180 };
  const frame = { left: 75, top: 125, right: 125, bottom: 175 };
  const zoom = zoomToFrame(frame, box, 1, { x: 0, y: 0 }, Math.PI / 2, 0.9, 100, area);
  close(zoom.scale, 2.88);
  close(zoom.x, -50);
  close(zoom.y, 0);
});

test('a crop pushed past the safe area zooms out to fit it at its size, centred', () => {
  const box = { width: 200, height: 300 };
  const area = { left: 20, top: 20, right: 180, bottom: 180 };
  const grown = { left: 50, top: 50, right: 150, bottom: 250 };
  const zoom = zoomOutToFrame(grown, box, 1, { x: 0, y: 0 }, 0, 0.5, area);
  close(zoom.scale, 0.8);
  close(zoom.x, 0);
  close(zoom.y, -50);
  assert.equal(
    zoomOutToFrame({ left: 75, top: 125, right: 125, bottom: 175 }, box, 1, { x: 0, y: 0 }, 0, 0.5, area),
    null,
  );
});

test('a crop pushed past the safe area slides back inside it on any side, without zooming', () => {
  const area = { left: 20, top: 20, right: 180, bottom: 180 };
  const below = slideFrameInto({ left: 50, top: 100, right: 150, bottom: 250 }, 1, { x: 0, y: 0 }, 0, area);
  close(below.x, 0);
  close(below.y, -70);
  const aboveLeft = slideFrameInto({ left: 0, top: -10, right: 100, bottom: 90 }, 1, { x: 5, y: 5 }, 0, area);
  close(aboveLeft.x, 25);
  close(aboveLeft.y, 35);
  const turned = slideFrameInto({ left: 50, top: 100, right: 150, bottom: 190 }, 1, { x: 0, y: 0 }, Math.PI / 2, area);
  close(turned.x, -10);
  close(turned.y, 0);
  assert.equal(slideFrameInto({ left: 75, top: 125, right: 125, bottom: 175 }, 1, { x: 0, y: 0 }, 0, area), null);
  assert.equal(slideFrameInto({ left: 0, top: 0, right: 200, bottom: 200 }, 1, { x: 0, y: 0 }, 0, area), null);
});

test('a photo that already fits above the gestures row needs no zoom floor', () => {
  const box = { width: 300, height: 400 };
  assert.equal(cropZoomFloor({ width: 200, height: 100 }, box, 50, 0), 1);
});

test('a tall photo gets a floor that fits it above the gestures row, at any turn', () => {
  const photo = { width: 400, height: 1200 };
  const box = { width: 300, height: 400 };
  const cover = 60;
  for (const turn of [0, Math.PI / 6]) {
    const floor = cropZoomFloor(photo, box, cover, turn);
    assert.ok(floor < 1);
    const centre = panToCentreAboveBar(cover, turn);
    const bounds = photoBoundsOnScreen(photo, box, floor, centre, turn);
    assert.ok(bounds.top >= -1e-9, `top ${bounds.top}`);
    assert.ok(bounds.bottom <= box.height - cover + 1e-9, `bottom ${bounds.bottom}`);
  }
});

test('the centring pan puts the photo centre in the middle of the area above the gestures row', () => {
  const photo = { width: 400, height: 300 };
  const box = { width: 300, height: 400 };
  const cover = 60;
  for (const turn of [0, Math.PI / 6]) {
    const bounds = photoBoundsOnScreen(photo, box, 0.8, panToCentreAboveBar(cover, turn), turn);
    close((bounds.left + bounds.right) / 2, box.width / 2);
    close((bounds.top + bounds.bottom) / 2, (box.height - cover) / 2);
  }
});

test('zooming around a corner keeps that corner on the same spot of the preview', () => {
  const photo = { width: 400, height: 300 };
  const box = { width: 300, height: 400 };
  for (const turn of [0, Math.PI / 6]) {
    const scale = 1.4;
    const translate = { x: 12, y: -30 };
    const fraction = { x: 0.2, y: 0.85 };
    const next = 0.9;
    const pan = panToKeepPoint(fraction, photo, box, scale, translate, turn, next);
    const before = cropToScreen(
      { left: fraction.x, top: fraction.y, right: fraction.x, bottom: fraction.y },
      photo,
      box,
      scale,
      translate,
      turn,
    );
    const after = cropToScreen(
      { left: fraction.x, top: fraction.y, right: fraction.x, bottom: fraction.y },
      photo,
      box,
      next,
      pan,
      turn,
    );
    close(after.left, before.left);
    close(after.top, before.top);
  }
});
