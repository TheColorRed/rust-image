import { Canvas, DropShadow, LayerEffects, type CanvasLike, type LayerLike } from '@abra/core/canvas';
import { Color, type ColorLike } from '@abra/core/color';
import { quadCorners } from '@abra/core/geometry';
import { Image, ImageLike } from '@abra/core/image';
import { app, inspector, release, seededRandom, type PickedFile } from '@alakazam/sdk';
import { autoLayoutStyle } from './styles/auto-layout';
import { gridStyle } from './styles/grid';
import { layeredGridStyle } from './styles/layered-grid';
import { randomStyle } from './styles/random';
import { SEED, type CollageStyle } from './styles/style';

/** The ways to lay the pictures out. A new style is a new file in `styles` and a line here. */
const STYLES: CollageStyle[] = [gridStyle, layeredGridStyle, randomStyle, autoLayoutStyle];

let style: CollageStyle = STYLES[0];
let collageImages: PickedFile[] = [];
/** What the canvas below was made from, so it is made again only when something that changes it changes. */
let builtFrom = '';
let canvas: CanvasLike | null = null;
/** The pictures read so far, by path, kept between builds so a change to the frame, the shadow or the layout does not read them again. A picture is let go when it is no longer picked. */
const photoCache = new Map<string, Image | ImageLike>();
/** Numbers each build, so one that finishes after a newer one started, or after the workspace closed, is known to be out of date. */
let building = 0;
/** The cells of the canvas, which the rotation turns. */
let cells: CanvasLike[] = [];
let rotation: [number, number] = [0, 0];
let scale: [number, number] = [1, 1];
/** The scale the cells were made at, so a drag of the slider can stretch them by the difference until they are made again. */
let builtScale: [number, number] = [1, 1];
let imageSize: [number, number] = [1024, 1024];
/** The size of the canvas that is showing, which is `imageSize` unless the style trimmed the empty edges off it. */
let canvasSize: [number, number] = imageSize;
/** Whether a style that can trim makes the collage only as big as its cells, with no empty edges. */
let trimEdges = true;
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

/** The words written over the collage, in a font downloaded on the Fonts screen, or 'None' for no font. */
let caption = '';
let captionFont = 'None';
/** The font of the words on the Polaroids, which is its own setting and not the font of the words over the collage. */
let polaroidFont = 'None';
/** The size of the letters as a share of the shorter side of the collage, as a percent. */
let captionSize = 8;
/** The colors to choose from, by name. A new color is a line here. */
const CAPTION_COLORS: Record<string, () => ColorLike> = {
  White: () => Color.white(),
  Black: () => Color.black(),
  Red: () => Color.red(),
  Yellow: () => Color.fromRgba(255, 214, 10, 255),
  Blue: () => Color.fromRgba(10, 132, 255, 255),
};
let captionColor = 'White';
const CAPTION_POSITIONS = ['Top', 'Middle', 'Bottom'];
let captionPosition = 'Bottom';
/** The layer on top of the collage that holds the words. It is there whenever the collage is; it is hidden when there is nothing to write. */
let textLayer: LayerLike | null = null;

/** What the text layer shows now, so it is made again only when something that changes it changes. */
let writtenCaption = '';

/**
 * Writes the caption on the collage that is showing. Only the text layer changes (its picture and place), so typing, or a change to the
 * font, size, color or position, does not make the cells again. Setting the text is a native call that blocks the JavaScript thread, so
 * it is not done for each move of a slider: the controls only set the values, and the editor runs `previewCanvas` once they settle.
 */
const writeCaption = () => {
  if (!canvas || !textLayer) return;
  const wanted = [caption, captionFont, captionSize, captionColor, captionPosition, canvasSize.join('x')].join('\n');
  if (wanted === writtenCaption) return;
  writtenCaption = wanted;
  const [width, height] = canvasSize;
  const shorter = Math.min(width, height);
  const color = CAPTION_COLORS[captionColor]();
  const image = app.fonts.renderText({
    text: caption,
    family: captionFont,
    size: (shorter * captionSize) / 100,
    color,
    width: width * 0.9,
    align: 'center',
  });
  release(color);
  if (!image) {
    textLayer.setVisible(false);
    return;
  }
  const [textWidth, textHeight] = [image.width(), image.height()];
  const margin = shorter * 0.04;
  const top =
    captionPosition === 'Top'
      ? margin
      : captionPosition === 'Bottom'
        ? height - textHeight - margin
        : (height - textHeight) / 2;
  textLayer.setImage(image);
  textLayer.setPosition(Math.round((width - textWidth) / 2), Math.round(top));
  textLayer.setVisible(true);
  release(image);
};

const toPath = (uri: string) => decodeURIComponent(uri.replace(/^file:\/\//, ''));

/** The cell picked on the preview, by its place in the collage, or null. While one is picked the inspector shows that cell's settings in place of the collage's. */
let selected: number | null = null;
/** Words and when they were written, so of the layers that reach a cell the newest is what it says. */
type Said = { words: string; stamp: number };
let stamp = 0;
/** What one cell says, by `captionKeys`. */
const cellWords = new Map<string, Said>();
/** What every copy of a picture says, by the picture. Only while the scope is Same. */
const sameWords = new Map<string, Said>();
/** What every cell says. Only while the scope is All. */
let allWords: Said | null = null;
const said = (words: string): Said => ({ words, stamp: ++stamp });

/** What a cell says on the bottom of its Polaroid: the newest of its own words, its picture's and everyone's. */
const wordsOf = (index: number) =>
  [cellWords.get(captionKeys[index]), sameWords.get(sources[index]), allWords].reduce<Said | undefined>(
    (newest, each) => (each && (!newest || each.stamp > newest.stamp) ? each : newest),
    undefined,
  )?.words ?? '';
/** Names each cell for `cellWords`: its picture, and how many cells before it show the same picture. A new layout of the same pictures gives the pictures their words back. */
let captionKeys: string[] = [];
/** The picture a cell was swapped for, by the cell's name in `captionKeys`. A cell not in it shows the picture the layout gave it. */
const swaps = new Map<string, string>();

/** Keeps what the cells hold that no control does (their swaps and their words) with the project, so it is there when the project opens. */
const keepCells = () =>
  app.saved.set('cells', {
    swaps: [...swaps],
    cellWords: [...cellWords],
    sameWords: [...sameWords],
    allWords,
    stamp,
    scope: captionScope,
  });

/** Puts back what `keepCells` kept, if the project has any. */
const restoreCells = () => {
  const kept = app.saved.get('cells') as
    | {
        swaps: [string, string][];
        cellWords: [string, Said][];
        sameWords: [string, Said][];
        allWords: Said | null;
        stamp: number;
        scope: string;
      }
    | undefined;
  if (!kept) return;
  kept.swaps.forEach(([slot, uri]) => swaps.set(slot, uri));
  kept.cellWords.forEach(([slot, words]) => cellWords.set(slot, words));
  kept.sameWords.forEach(([uri, words]) => sameWords.set(uri, words));
  allWords = kept.allWords;
  stamp = kept.stamp;
  captionScope = kept.scope;
};
/** Which cells the words typed on the picked cell go to: only it (Current), every copy of its picture (Same), or every cell (All). */
let captionScope = 'Current';
/** For each cell, the layer its words are on and the size of the cell and its edges, or null for a cell with no strip to write on. Only a Polaroid has one. */
let strips: ({ layer: LayerLike; width: number; height: number; side: number; bottom: number } | null)[] = [];
/** The picture each cell shows, by the cell's place in the collage. Cells with the same one are copies of each other. */
let sources: string[] = [];
/** What each strip shows now, so it is made again only when something that changes it changes. */
const writtenStrips = new Map<number, string>();

/** How much of the strip the letters fill, as a share of its height. */
const STRIP_TEXT = 0.5;

/** Writes each cell's words on the strip of its Polaroid. Only the strip's layer changes, so typing does not make the cells again. */
const writeStrips = () => {
  strips.forEach((strip, index) => {
    if (!strip) return;
    const words = wordsOf(index);
    const wanted = [words, polaroidFont, strip.bottom, strip.width].join('\n');
    if (writtenStrips.get(index) === wanted) return;
    writtenStrips.set(index, wanted);
    const ink = Color.black();
    const image = app.fonts.renderText({
      text: words,
      family: polaroidFont,
      size: strip.bottom * STRIP_TEXT,
      color: ink,
      width: strip.width - 2 * strip.side,
      align: 'center',
    });
    release(ink);
    if (!image) {
      strip.layer.setVisible(false);
      return;
    }
    // Centered in the strip by the size of the words, which is the size of the picture the library made of them.
    const [wordsWidth, wordsHeight] = [image.width(), image.height()];
    strip.layer.setImage(image);
    strip.layer.setPosition(
      Math.round((strip.width - wordsWidth) / 2),
      Math.round(strip.height - strip.bottom + (strip.bottom - wordsHeight) / 2),
    );
    strip.layer.setVisible(true);
    release(image);
  });
};

/** Draws the frame round the picked cell, or takes it away when none is picked. The app keeps it on the cell as the preview is zoomed, moved and turned. */
const drawSelection = () => {
  const cell = selected === null ? undefined : cells[selected];
  if (!cell) {
    app.gizmos.remove('selection');
    return;
  }
  const corners = quadCorners(cell.quadInParent()).map(corner => [corner.x, corner.y]);
  app.gizmos.setBox('selection', corners);
};

/** Picks a cell, or none, and shows its frame and its settings. */
const selectCell = (index: number | null) => {
  selected = index;
  drawSelection();
  inspector.refresh();
  // On a narrow screen the settings of the cell are behind a button, so a tap on a cell brings them up. Letting go leaves the panel as it is.
  if (index !== null) inspector.open();
};

// A tap on the preview picks the cell that is on top there; a tap off every cell lets go.
app.events.taps.subscribe(({ imageX, imageY }) => {
  if (!canvas) return;
  const found = imageX === null || imageY === null ? undefined : canvas.childAt(imageX, imageY);
  const index = found ? cells.findIndex(cell => cell.id() === found.id()) : -1;
  release(found);
  selectCell(index >= 0 ? index : null);
});

/** How much each cell is scaled at a setting of the Scale slider, in the order of the cells (chosen once so a drag does not shuffle them). */
const factorsAt = (range: [number, number]) => {
  const rand = seededRandom(`${SEED}:scale`);
  return cells.map(() => (style.scales ? range[0] + (range[1] - range[0]) * rand.next() : 1));
};

/**
 * Turns every cell to where the rotation slider says, and while the Scale slider is moving, stretches the cells from the size they were
 * made at so the preview follows live. The stretch is soft, and goes when the cells are made again at the new scale, from the original pictures.
 */
const shape = () => {
  const rotateRand = seededRandom(`${SEED}:rotate`);
  const [now, built] = [factorsAt(scale), factorsAt(builtScale)];
  cells.forEach((cell, index) => {
    // The random number is drawn either way, so the turns of the other styles do not change.
    const turn = rotation[0] + (rotation[1] - rotation[0]) * rotateRand.next();
    cell.setRotation(style.rotates === false ? 0 : turn);
    if (style.scaleFrom === 'middle') cell.setScale(now[index] / built[index]);
  });
  drawSelection();
};
export const inspectorGUI = () => {
  if (selected !== null && selected < cells.length) cellInspector(selected);
  else mainInspector();
};

/**
 * Puts the words where the scope says: on the picked cell, on every copy of its picture, or on every cell. A scope that reaches further
 * than the last one writes a layer over the cells, and one that reaches less takes the wider layers away, so the other cells say again
 * what they said before. The picked cell keeps its words either way.
 */
const applyWords = (index: number, words: string) => {
  cellWords.set(captionKeys[index], said(words));
  if (captionScope === 'Same') sameWords.set(sources[index], said(words));
  else sameWords.clear();
  allWords = captionScope === 'All' ? said(words) : null;
  keepCells();
};

/** The settings of one picked cell. */
const cellInspector = (index: number) => {
  const { header, select, fontPicker, text, pictures } = inspector;
  header(`Image ${index + 1}`, { back: () => selectCell(null) });
  // Every picked picture, to put in this cell in place of the one it shows.
  pictures('Swap Image', {
    files: collageImages,
    onPick: picked => {
      swaps.set(captionKeys[index], collageImages[picked].uri);
      keepCells();
    },
  });
  if (swaps.has(captionKeys[index])) {
    select('Original Image', {
      options: ['Put Back'],
      momentary: true,
      onChange: () => {
        swaps.delete(captionKeys[index]);
        keepCells();
      },
    });
  }
  if (!strips[index]) {
    // Words go on the strip at the bottom of a Polaroid, so the frame is offered here.
    header('Caption');
    select('Frame', {
      options: Object.keys(FRAMES),
      value: frame,
      onChange: value => {
        frame = value;
      },
    });
    return;
  }
  header('Caption');
  const fonts = app.fonts.installed();
  fontPicker('Caption Font', {
    value: fonts.includes(polaroidFont) ? polaroidFont : undefined,
    onPick: family => {
      polaroidFont = family;
    },
  });
  text('Caption Words', {
    value: wordsOf(index),
    placeholder: fonts.length > 0 ? 'Write something' : 'Download a font on the Fonts tab',
    onChange: value => applyWords(index, value),
  });
  select('Apply words to', {
    options: ['Current', 'Same', 'All'],
    value: captionScope,
    onChange: value => {
      captionScope = value;
      applyWords(index, wordsOf(index));
    },
  });
};

/** The settings of the whole collage. */
const mainInspector = () => {
  const { header, filePicker, slider, size, select, toggle, fontPicker, text } = inspector;

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
  if (style.trims) {
    toggle('Trim Edges', {
      value: trimEdges,
      onChange: value => {
        // The collage is made again at its new size, which `previewCanvas` does when the editor runs it again.
        trimEdges = value;
      },
    });
  }

  header('Options');
  if (style.rotates !== false) {
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
  }
  if (style.scales) {
    slider('Scale', {
      min: 0.1,
      max: 3,
      range: true,
      step: 0.1,
      // The cells follow the slider by stretching, then are made again from the original pictures when it stops, so they end sharp.
      rebuild: true,
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
  if (style.shadows !== false) {
    toggle('Shadow', {
      value: shadow,
      onChange: value => {
        // A shadow is part of each cell, so the cells are made again, which `previewCanvas` does when the editor runs it again.
        shadow = value;
      },
    });
  }

  header('Text');
  const fonts = ['None', ...app.fonts.installed()];
  fontPicker('Font', {
    // A font that was deleted since it was chosen is no longer one.
    value: fonts.includes(captionFont) && captionFont !== 'None' ? captionFont : undefined,
    onPick: family => {
      captionFont = family;
    },
  });
  text('Words', {
    value: caption,
    placeholder: fonts.length > 1 ? 'Write something' : 'Download a font on the Fonts tab',
    onChange: value => {
      caption = value;
    },
  });
  slider('Text Size', {
    min: 2,
    max: 30,
    step: 1,
    // The text is made once the slider stops, not on every move.
    rebuild: true,
    value: captionSize,
    onChange: value => {
      captionSize = value;
    },
  });
  select('Text Color', {
    options: Object.keys(CAPTION_COLORS),
    value: captionColor,
    onChange: value => {
      captionColor = value;
    },
  });
  select('Text Position', {
    options: CAPTION_POSITIONS,
    value: captionPosition,
    onChange: value => {
      captionPosition = value;
    },
  });
};

/** Lets go of the canvas and its cells, and calls a build that is still running out of date. */
const free = () => {
  building++;
  release(textLayer, ...strips.map(strip => strip?.layer), canvas, ...cells, ...photoCache.values());
  photoCache.clear();
  textLayer = null;
  strips = [];
  sources = [];
  captionKeys = [];
  writtenStrips.clear();
  selected = null;
  drawSelection();
  writtenCaption = '';
  canvas = null;
  cells = [];
  builtFrom = '';
};

/**
 * The picked pictures in the cells the chosen style gives. Made once for a set of pictures, a style and its settings. The pictures are
 * read by the library off the JavaScript thread (`Image.readAsync`), each file once however many cells use it and however many times the
 * cells are made again (`photoCache`), so the app keeps drawing while they load. A cell holds its picture at full size in a canvas of its own, and that canvas is scaled to cover the cell (or, when the
 * cell keeps its aspect ratio, to fit in it), so nothing is resized pixel by pixel here. The Scale slider is part of the layout: a bigger cell is made again from the full-size picture. The canvas that is on show stays until the new
 * one is ready.
 */
export const previewCanvas = async () => {
  const key = [
    style.name,
    style.key(),
    imageSize.join('x'),
    style.trims && trimEdges,
    style.scales ? scale.join('x') : '',
    frame,
    shadow && style.shadows !== false,
    [...swaps].map(([slot, uri]) => `${slot}>${uri}`).join(','),
    ...collageImages.map(file => file.uri),
  ].join('\n');
  if (key === builtFrom) {
    // Only the words may have changed.
    writeCaption();
    writeStrips();
    return canvas;
  }
  if (!collageImages.length) {
    free();
    return null;
  }

  const build = ++building;
  /** Reads the pictures that are not read yet, off the JavaScript thread. False when something newer started meanwhile or the workspace closed, and the newer build reads what it is missing. */
  const load = async (wanted: string[]) => {
    const read = await Promise.all(
      wanted.filter(path => !photoCache.has(path)).map(async path => [path, await Image.readAsync(path)] as const),
    );
    if (build !== building) {
      release(...read.map(([, photo]) => photo));
      return false;
    }
    read.forEach(([path, photo]) => photoCache.set(path, photo));
    return true;
  };
  // A style that lays the pictures out by their proportions needs them all read first.
  let shapes: number[] = [];
  if (style.needsShapes) {
    if (!(await load(collageImages.map(file => toPath(file.uri))))) return canvas;
    shapes = collageImages.map(file => {
      const photo = photoCache.get(toPath(file.uri))!;
      return photo.width() / photo.height();
    });
  }
  // A style whose places touch gives every cell one border width, taken from the size of an average cell, and is told it so it can leave
  // room for the borders in the places it makes.
  const card = FRAMES[frame];
  const averageCell = Math.sqrt((imageSize[0] * imageSize[1]) / Math.max(1, collageImages.length));
  const sharedSide = card && style.sharedBorder ? Math.round(averageCell * card.side) : 0;
  const border = sharedSide ? { side: sharedSide, bottom: Math.round(sharedSide * (card!.bottom / card!.side)) } : undefined;
  const placed = style.layout(collageImages.length, imageSize, shapes, border);
  // With Trim Edges the collage is only as big as its cells, so there are no empty bands. A cell with shared borders reaches half a border past its place.
  let size = imageSize;
  let layout = placed;
  if (style.trims && trimEdges && placed.length > 0) {
    const reach = Math.round(sharedSide / 2);
    const left = Math.min(...placed.map(spot => spot.x)) - reach;
    const top = Math.min(...placed.map(spot => spot.y)) - reach;
    const right = Math.max(...placed.map(spot => spot.x + spot.width)) + (sharedSide - reach);
    const bottom = Math.max(...placed.map(spot => spot.y + spot.height)) + (sharedSide - reach);
    size = [Math.max(1, right - left), Math.max(1, bottom - top)];
    layout = placed.map(spot => ({ ...spot, x: spot.x - left, y: spot.y - top }));
  }
  // A cell is named by the picture the layout gave it and how many cells before it were given the same, so it keeps its swap and its words
  // when the layout changes. What it shows is that picture, or the one it was swapped for if that is still picked.
  const given = new Map<string, number>();
  const slots = layout.map(spot => {
    const uri = collageImages[spot.image].uri;
    const nth = given.get(uri) ?? 0;
    given.set(uri, nth + 1);
    const swapped = swaps.get(`${uri}#${nth}`);
    return { slot: `${uri}#${nth}`, shown: swapped && collageImages.some(file => file.uri === swapped) ? swapped : uri };
  });
  const paths = [...new Set(slots.map(each => toPath(each.shown)))];
  if (!(await load(paths))) return canvas;
  const photos = photoCache;

  const made = Canvas.newBlank('Collage', size[0], size[1]);
  const madeStrips: typeof strips = [];
  const madeSources: string[] = [];
  const scaleRand = seededRandom(`${SEED}:scale`);
  for (const [at, spot] of layout.entries()) {
    // The Scale slider sizes the place itself, so the picture is shrunk once, straight from the original. A place grows about its middle
    // or, for a style that says so, about its top-left corner.
    const factor = style.scales ? scale[0] + (scale[1] - scale[0]) * scaleRand.next() : 1;
    const [grownWidth, grownHeight] = [
      Math.max(1, Math.round(spot.width * factor)),
      Math.max(1, Math.round(spot.height * factor)),
    ];
    const middle = style.scaleFrom !== 'corner';
    const place = {
      ...spot,
      width: grownWidth,
      height: grownHeight,
      x: middle ? spot.x + Math.round((spot.width - grownWidth) / 2) : spot.x,
      y: middle ? spot.y + Math.round((spot.height - grownHeight) / 2) : spot.y,
    };
    const photo = photos.get(toPath(slots[at].shown))!;
    const [photoWidth, photoHeight] = [photo.width(), photo.height()];
    // The card is the cell, and the picture goes in the space inside its white edges (all of the cell when there is no card).
    // With shared borders the cell reaches half a border width past its place on each side, so where two cells meet their borders lie over
    // each other and are one border. The place was made to hold the picture and that one border, so the cell fills it, with no gap.
    const shared = sharedSide;
    if (shared) {
      place.x -= Math.round(shared / 2);
      place.y -= Math.round(shared / 2);
      place.width += shared;
      place.height += shared;
      place.keepAspect = false;
    }
    const side = shared || (card ? Math.round(Math.min(place.width, place.height) * card.side) : 0);
    // The bottom edge keeps its share of the side, so a Polaroid's strip is as thick as the others', and a plain border is the same all round.
    const bottom = shared
      ? Math.round(shared * (card!.bottom / card!.side))
      : card
        ? Math.round(Math.min(place.width, place.height) * card.bottom)
        : 0;
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
    if (shadow && style.shadows !== false) {
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
    // A Polaroid's thick edge is a strip to write on. A cell draws its own layers over its canvases, so the words are one, hidden until there are some.
    let strip: (typeof strips)[number] = null;
    if (card && card.bottom > card.side * 2) {
      const clear = Color.fromRgba(0, 0, 0, 0);
      const blank = Image.newFromColor(1, 1, clear);
      strip = {
        layer: cell.addLayerFromImage('caption', blank, undefined),
        width: cellWidth,
        height: cellHeight,
        side,
        bottom,
      };
      release(clear, blank);
    }
    madeStrips.push(strip);
    madeSources.push(slots[at].shown);
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

  // The words go on last, so they are over every cell. The layer is hidden until there is something to write.
  const clear = Color.fromRgba(0, 0, 0, 0);
  const blank = Image.newFromColor(1, 1, clear);
  const text = made.addLayerFromImage('text', blank, undefined);
  release(clear, blank);
  release(textLayer, ...strips.map(each => each?.layer), canvas, ...cells);
  canvas = made;
  canvasSize = size;
  textLayer = text;
  cells = made.canvases();
  strips = madeStrips;
  sources = madeSources;
  captionKeys = slots.map(each => each.slot);
  writtenStrips.clear();
  // The cells are new, so a picked cell is the one at the same place, if there is still one.
  if (selected !== null && selected >= cells.length) selected = null;
  builtScale = scale;
  shape();
  builtFrom = key;
  writtenCaption = '';
  writeCaption();
  writeStrips();
  return canvas;
};

/** The workspace closed: frees the canvas and its cells. */
export const dispose = free;

// A project that was saved has its swaps and words ready before the first preview is made.
restoreCells();
