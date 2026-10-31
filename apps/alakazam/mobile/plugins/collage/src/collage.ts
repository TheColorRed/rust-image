import { Canvas, DropShadow, LayerEffects, type CanvasLike } from '@abra/core/canvas';
import { Color } from '@abra/core/color';
import { Image } from '@abra/core/image';
import { inspector, release, seededRandom, type PickedFile } from '@alakazam/sdk';
import { gridStyle } from './styles/grid';
import { layeredGridStyle } from './styles/layered-grid';
import { randomStyle } from './styles/random';
import { SEED, type CollageStyle } from './styles/style';

/** The ways to lay the pictures out. A new style is a new file in `styles` and a line here. */
const STYLES: CollageStyle[] = [gridStyle, layeredGridStyle, randomStyle];

let style: CollageStyle = STYLES[0];
let collageImages: PickedFile[] = [];
/** What the canvas below was made from, so it is made again only when something that changes it changes. */
let builtFrom = '';
let canvas: CanvasLike | null = null;
/** The pictures read so far, by path, kept between builds so a change to the frame, the shadow or the layout does not read them again. A picture is let go when it is no longer picked. */
const photoCache = new Map<string, Image>();
/** Numbers each build, so one that finishes after a newer one started, or after the workspace closed, is known to be out of date. */
let building = 0;
/** The cells of the canvas, and how far through the rotation range each one turns (0 to 1, chosen once so a drag does not shuffle them). */
let cells: CanvasLike[] = [];
/** Where each cell sits and how big it is when it is not scaled, in the same order as `cells`. The library sizes a cell that keeps its aspect ratio from its picture, so this is read from the cells. */
let homes: { x: number; y: number; width: number; height: number }[] = [];
let rotation: [number, number] = [0, 0];
let scale: [number, number] = [1, 1];
let imageSize: [number, number] = [1024, 1024];
/**
 * The white card each picture is put on, by name, as how wide its edges are: a share of the shorter side of the cell, at the sides and
 * top and at the bottom. A border is even all around; a Polaroid is thin at the sides and top and thick at the bottom. A new frame is a line here.
 */
const FRAMES: Record<string, { side: number; bottom: number } | undefined> = {
  None: undefined,
  Border: { side: 0.035, bottom: 0.035 },
  Polaroid: { side: 0.035, bottom: 0.12 },
};
let frame = 'None';
/** Drops a shadow under each cell. */
let shadow = false;
/** How far the shadow falls from its cell and how soft it is, as a share of the shorter side of the cell. */
const SHADOW_DISTANCE = 0.02;
const SHADOW_SIZE = 0.03;

const toPath = (uri: string) => decodeURIComponent(uri.replace(/^file:\/\//, ''));

/** Turns and scales every cell to where the sliders say. */
const shape = () => {
  const scaleRand = seededRandom(`${SEED}:scale`);
  const rotateRand = seededRandom(`${SEED}:rotate`);

  cells.forEach((cell, index) => {
    const factor = style.scales ? scale[0] + (scale[1] - scale[0]) * scaleRand.next() : 1;
    cell.setRotation(rotation[0] + (rotation[1] - rotation[0]) * rotateRand.next());
    cell.setScale(factor);
    if (style.scaleFrom === 'corner') {
      // The canvas scales about its middle, so it moves by half of its growth to keep its top-left corner at its place.
      const home = homes[index];
      cell.setPosition(
        Math.round(home.x + (home.width * (factor - 1)) / 2),
        Math.round(home.y + (home.height * (factor - 1)) / 2),
      );
    }
  });
};
export const inspectorGUI = () => {
  const { header, filePicker, slider, size, select, toggle } = inspector;

  header('Image');
  size('Image Size', {
    value: imageSize,
    min: 1,
    onChange: value => {
      // A new size is a new canvas, which `previewCanvas` makes when the editor runs it again.
      imageSize = value;
    },
  });
  filePicker('Collage Images', {
    source: 'both',
    multiple: true,
    thumbs: true,
    onPick: files => {
      collageImages = files;
    },
  });

  header('Layout');
  select('Style', {
    options: STYLES.map(each => each.name),
    value: style.name,
    onChange: name => {
      style = STYLES.find(each => each.name === name) ?? style;
    },
  });
  style.controls();

  header('Options');
  slider('Rotation', {
    min: -360,
    max: 360,
    step: 1,
    range: true,
    value: rotation,
    onChange: value => {
      rotation = value;
      // Only the cells turn: each is set in the canvas the preview already shows, and the engine draws it.
      shape();
    },
  });
  if (style.scales) {
    slider('Scale', {
      min: 0.1,
      max: 3,
      range: true,
      step: 0.1,
      value: scale,
      onChange: value => {
        scale = value;
        shape();
      },
    });
  }

  header('Effects');
  select('Frame', {
    options: Object.keys(FRAMES),
    value: frame,
    onChange: value => {
      // A frame is part of each cell, so the cells are made again, which `previewCanvas` does when the editor runs it again.
      frame = value;
    },
  });
  toggle('Shadow', {
    value: shadow,
    onChange: value => {
      // A shadow is part of each cell, so the cells are made again, which `previewCanvas` does when the editor runs it again.
      shadow = value;
    },
  });
};

/** Lets go of the canvas and its cells, and calls a build that is still running out of date. */
const free = () => {
  building++;
  release(canvas, ...cells, ...photoCache.values());
  photoCache.clear();
  canvas = null;
  cells = [];
  homes = [];
  builtFrom = '';
};

/**
 * The picked pictures in the cells the chosen style gives. Made once for a set of pictures, a style and its settings. The pictures are
 * read by the library off the JavaScript thread (`Image.readAsync`), each file once however many cells use it and however many times the
 * cells are made again (`photoCache`), so the app keeps drawing while they load. A cell holds its picture at full size in a canvas of its own, and that canvas is scaled to cover the cell (or, when the
 * cell keeps its aspect ratio, to fit in it), so nothing is resized pixel by pixel here. The canvas that is on show stays until the new
 * one is ready.
 */
export const previewCanvas = async () => {
  const key = [
    style.name,
    style.key(),
    imageSize.join('x'),
    frame,
    shadow,
    ...collageImages.map(file => file.uri),
  ].join('\n');
  if (key === builtFrom) return canvas;
  if (!collageImages.length) {
    free();
    return null;
  }

  const build = ++building;
  const layout = style.layout(collageImages.length, imageSize);
  const paths = [...new Set(layout.map(place => toPath(collageImages[place.image].uri)))];
  const missing = paths.filter(path => !photoCache.has(path));
  const read = await Promise.all(missing.map(async path => [path, await Image.readAsync(path)] as const));
  if (build !== building) {
    // Something newer started while the pictures were read, or the workspace closed. The newer build reads what it is missing.
    release(...read.map(([, photo]) => photo));
    return canvas;
  }
  read.forEach(([path, photo]) => photoCache.set(path, photo));
  const photos = photoCache;

  const made = Canvas.newBlank('Collage', imageSize[0], imageSize[1]);
  for (const place of layout) {
    const photo = photos.get(toPath(collageImages[place.image].uri))!;
    const [photoWidth, photoHeight] = [photo.width(), photo.height()];
    // The card is the cell, and the picture goes in the space inside its white edges (all of the cell when there is no card).
    const card = FRAMES[frame];
    const side = card ? Math.round(Math.min(place.width, place.height) * card.side) : 0;
    const bottom = card ? Math.round(Math.min(place.width, place.height) * card.bottom) : 0;
    const boxWidth = Math.max(1, place.width - 2 * side);
    const boxHeight = Math.max(1, place.height - side - bottom);
    const fit = Math.min(boxWidth / photoWidth, boxHeight / photoHeight);
    const cover = Math.max(boxWidth / photoWidth, boxHeight / photoHeight);
    // Keeping the aspect ratio, the picture's space shrinks to the picture scaled to fit, and the cell, with its edges, sits in the middle
    // of its place. Otherwise the space fills the box and the picture, scaled to cover it, is cropped.
    const width = place.keepAspect ? Math.min(boxWidth, Math.max(1, Math.round(photoWidth * fit))) : boxWidth;
    const height = place.keepAspect ? Math.min(boxHeight, Math.max(1, Math.round(photoHeight * fit))) : boxHeight;
    const cellWidth = width + 2 * side;
    const cellHeight = height + side + bottom;

    const cell = Canvas.newBlank('Cell', cellWidth, cellHeight);
    if (shadow) {
      const reach = Math.min(cellWidth, cellHeight);
      const drop = new DropShadow().withDistance(reach * SHADOW_DISTANCE).withSize(reach * SHADOW_SIZE);
      const effects = new LayerEffects().withDropShadow(drop);
      cell.setEffects(effects);
      release(drop, effects);
    }
    if (card) {
      // A canvas draws its child canvases first and its own layers over them, so the card is a child of its own, added before the picture's.
      const backing = Canvas.newBlank('Card', cellWidth, cellHeight);
      const white = Image.newFromColor(cellWidth, cellHeight, Color.white());
      release(
        backing.addLayerFromImage('card', white, undefined),
        white,
        cell.addCanvas(backing, { position: [0, 0] }),
        backing,
      );
    }
    const area = Canvas.newBlank('Photo area', width, height);
    const picture = Canvas.newBlank('Photo', photoWidth, photoHeight);
    const layer = picture.addLayerFromImage('photo', photo, undefined);
    const inArea = area.addCanvas(picture, undefined);
    inArea.setScale(place.keepAspect ? fit : cover);
    const inCell = cell.addCanvas(area, { position: [side, side] });
    const inCollage = made.addCanvas(cell, {
      position: [
        place.x + Math.trunc((place.width - cellWidth) / 2),
        place.y + Math.trunc((place.height - cellHeight) / 2),
      ],
    });
    release(layer, inArea, picture, inCell, area, inCollage, cell);
  }
  // The pictures that are no longer picked are let go; the others stay for the next build.
  for (const [path, photo] of photoCache) {
    if (paths.includes(path)) continue;
    release(photo);
    photoCache.delete(path);
  }

  release(canvas, ...cells);
  canvas = made;
  cells = made.canvases();
  homes = cells.map(cell => {
    const [x, y] = cell.position();
    const [width, height] = cell.dimensions();
    return { x, y, width, height };
  });
  shape();
  builtFrom = key;
  return canvas;
};

/** The workspace closed: frees the canvas and its cells. */
export const dispose = free;
