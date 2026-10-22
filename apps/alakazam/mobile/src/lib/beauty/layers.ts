import { type BlendMode } from '@abra/core/blend';
import { Canvas, type LayerLike } from '@abra/core/canvas';
import { type ColorLike } from '@abra/core/color';
import { Image, type ImageLike } from '@abra/core/image';
import { type MaskLike } from '@abra/core/mask';
import { owning, release } from '@/src/lib/native/native-handles';

/**
 * What a makeup tool paints on: the size of the photo it is for, and, once `paintSheet` has been called, the picture it painted. A
 * sheet is only as big as the part of the photo that was painted (a cheek, the lips), and not as big as the photo, because a sheet
 * the size of a large photo costs as much memory as the photo does.
 */
export type Painting = {
  /** The photo the sheet goes over, for a layer that is a changed part of it. It is not changed. */
  photo: ImageLike;
  width(): number;
  height(): number;
  /** What was painted, and where its top left corner goes on the photo. Nothing until something is painted. */
  painted?: { image: ImageLike; x: number; y: number };
};

/**
 * One layer to put over the photo: a sheet that `paint` draws on, laid over what is under it with `blend` and shown at `opacity`
 * (0 to 1).
 */
export type Sheet = {
  /** The layer's name in the canvas. */
  name: string;
  /** The group canvas the layer goes in. Makeup when not given. */
  group?: 'skin' | 'makeup';
  blend: BlendMode;
  opacity: number;
  /** The sheet is a changed copy of the photo laid over the skin, so it is cut from the photo with the earlier sheets already laid, and does not cover them. */
  overEarlier?: boolean;
  /** Paints the sheet. A sheet that paints nothing is left out of the canvas. */
  paint: (painting: Painting) => void;
};

/**
 * The photo with the sheets laid over it, as a new image the caller owns. The photo is the canvas's `base` layer and each sheet
 * is a layer above it, in order, placed where it was painted, so the photo itself is never changed and a sheet can be redrawn
 * without touching it.
 */
export const withLayers = (photo: ImageLike, sheets: Sheet[]): ImageLike => {
  const root = Canvas.newBlank('root', photo.width(), photo.height());
  const owned: unknown[] = [root];
  const laid: string[] = [];
  const started = Date.now();
  let paintMs = 0;
  try {
    // Each group is a child canvas of the root, the size of the photo and at its top left corner, with a layer per feature. A
    // group passes through, so its layers blend with what is under the group and not with an empty canvas of their own.
    const group = (name: string) => {
      const blank = Canvas.newBlank(name, photo.width(), photo.height());
      owned.push(blank);
      const child = root.addCanvas(blank, { position: [0, 0] });
      child.setPassThrough(true);
      owned.push(child);
      return child;
    };
    owned.push(group('base').addLayerFromImage('photo', photo, undefined));
    // Groups stack in this order, and one with no sheets is not made.
    const groups = new Map<NonNullable<Sheet['group']>, ReturnType<typeof group>>();
    for (const name of ['skin', 'makeup'] as const) {
      if (sheets.some(sheet => (sheet.group ?? 'makeup') === name)) groups.set(name, group(name));
    }
    for (const sheet of sheets) {
      const canvas = groups.get(sheet.group ?? 'makeup')!;
      const painting: Painting = { photo, width: () => photo.width(), height: () => photo.height() };
      const paintStarted = Date.now();
      sheet.paint(painting);
      paintMs += Date.now() - paintStarted;
      if (!painting.painted) continue;
      const { image, x, y } = painting.painted;
      owned.push(image);
      const layer: LayerLike = canvas.addLayerFromImage(sheet.name, image, undefined);
      owned.push(layer);
      layer.setPosition(x, y);
      layer.setBlendMode(sheet.blend);
      layer.setOpacity(sheet.opacity);
      laid.push(`${sheet.name} (${sheet.blend}, ${sheet.opacity.toFixed(2)}, ${image.width()}x${image.height()} at ${x},${y})`);
    }
    const composeStarted = Date.now();
    const flat = root.asImage();
    if (__DEV__) {
      console.debug(
        `[layers] ${photo.width()}x${photo.height()} canvas, base + ${laid.length} sheets:`,
        laid.join(', ') || 'none',
        `| paint ${paintMs.toFixed(0)} ms, compose ${(Date.now() - composeStarted).toFixed(0)} ms, total ${(Date.now() - started).toFixed(0)} ms`,
      );
    }
    return flat;
  } finally {
    // They are native objects the garbage collector cannot see the size of.
    let freed = 0;
    for (const item of owned) {
      try {
        release(item);
        freed++;
      } catch (error) {
        console.warn('[layers] could not free an item:', error);
      }
    }
    if (__DEV__) {
      frames++;
      notFreed += owned.length - freed;
      if (frames % 10 === 0) console.debug(`[layers] frame ${frames}: made ${owned.length}, freed ${freed}, never freed so far ${notFreed}`);
    }
  }
};

let frames = 0;
let notFreed = 0;

/**
 * Paints `color` wherever any of the `regions` reaches, as strongly as the region does there, on a sheet that is cut down to the
 * part of the photo they reach. The regions are masks the size of the photo, and are not changed. Nothing is painted when none of
 * them reaches anywhere.
 */
export const paintSheet = (painting: Painting, regions: MaskLike[], color: ColorLike) => {
  if (regions.length === 0) return;
  owning(keep => {
    const reach = regions.slice(1).reduce((all, region) => keep(all.maximum(region)), regions[0]);
    const bounds = reach.bounds(0);
    if (!bounds) return;
    const [x, y] = [Math.floor(bounds.x), Math.floor(bounds.y)];
    const part = {
      x,
      y,
      width: Math.min(painting.width(), Math.ceil(bounds.x + bounds.width)) - x,
      height: Math.min(painting.height(), Math.ceil(bounds.y + bounds.height)) - y,
    };
    if (part.width < 1 || part.height < 1) return;
    const image = Image.newFromColor(part.width, part.height, color);
    keep(reach.cropped(part)).applyToImage(image);
    painting.painted = { image, x, y };
  });
};
