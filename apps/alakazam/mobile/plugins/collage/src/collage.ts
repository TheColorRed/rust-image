import type { CanvasLike } from '@abra/core/canvas';
import { canvasOfCells, inspector, release, type PickedFile } from '@alakazam/plugin';
import { gridStyle } from './styles/grid';
import { layeredGridStyle } from './styles/layered-grid';
import { randomStyle } from './styles/random';
import type { CollageStyle } from './styles/style';

/** The ways to lay the pictures out. A new style is a new file in `styles` and a line here. */
const STYLES: CollageStyle[] = [gridStyle, layeredGridStyle, randomStyle];

let style: CollageStyle = STYLES[0];
let collageImages: PickedFile[] = [];
/** What the canvas below was made from, so it is made again only when something that changes it changes. */
let builtFrom = '';
let canvas: CanvasLike | null = null;
/** Numbers each build, so one that finishes after a newer one started, or after the workspace closed, is known to be out of date. */
let building = 0;
/** The cells of the canvas, and how far through the rotation range each one turns (0 to 1, chosen once so a drag does not shuffle them). */
let cells: CanvasLike[] = [];
/** Where each cell sits and how big it is when it is not scaled, in the same order as `cells`. The library sizes a cell that keeps its aspect ratio from its picture, so this is read from the cells. */
let homes: { x: number; y: number; width: number; height: number }[] = [];
let spins: number[] = [];
/** How far through the scale range each cell grows (0 to 1), chosen once like `spins`. */
let grows: number[] = [];
let rotation: [number, number] = [0, 0];
let scale: [number, number] = [1, 1];
let imageSize: [number, number] = [1024, 1024];

const toPath = (uri: string) => decodeURIComponent(uri.replace(/^file:\/\//, ''));

/** Turns and scales every cell to where the sliders say. */
const shape = () =>
  cells.forEach((cell, index) => {
    const factor = style.scales ? scale[0] + (scale[1] - scale[0]) * grows[index] : 1;
    cell.setRotation(rotation[0] + (rotation[1] - rotation[0]) * spins[index]);
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

export const inspectorGUI = () => {
  const { header, filePicker, slider, size, select } = inspector;

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
};

/** Lets go of the canvas and its cells, and calls a build that is still running out of date. */
const free = () => {
  building++;
  release(canvas, ...cells);
  canvas = null;
  cells = [];
  homes = [];
  spins = [];
  grows = [];
  builtFrom = '';
};

/**
 * The picked pictures in the cells the chosen style gives. Made once for a set of pictures, a style and its settings, by the library on
 * its worker threads (`canvasOfCells`), so the app keeps drawing while the pictures are read and scaled. The canvas that is on show stays
 * until the new one is ready.
 */
export const previewCanvas = async () => {
  const key = [style.name, style.key(), imageSize.join('x'), ...collageImages.map(file => file.uri)].join('\n');
  if (key === builtFrom) return canvas;
  if (!collageImages.length) {
    free();
    return null;
  }

  const build = ++building;
  const layout = style.layout(collageImages.length, imageSize);
  const made = await canvasOfCells(
    'Collage',
    imageSize[0],
    imageSize[1],
    layout.map(place => ({
      path: toPath(collageImages[place.image].uri),
      x: place.x,
      y: place.y,
      width: place.width,
      height: place.height,
      keepAspect: place.keepAspect ?? false,
    })),
  );
  if (build !== building) {
    // Something newer started while this was being made, or the workspace closed.
    release(made);
    return canvas;
  }

  release(canvas, ...cells);
  canvas = made;
  cells = made.canvases();
  homes = cells.map(cell => {
    const [x, y] = cell.position();
    const [width, height] = cell.dimensions();
    return { x, y, width, height };
  });
  spins = cells.map(() => Math.random());
  grows = cells.map(() => Math.random());
  shape();
  builtFrom = key;
  return canvas;
};

/** The workspace closed: frees the canvas and its cells. */
export const dispose = free;
