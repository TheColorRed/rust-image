import { autoTone, gradientMap, grayscale, levels } from '@abra/core/adjustments';
import { blend, BlendMode } from '@abra/core/blend';
import { Color } from '@abra/core/color';
import { Fill, fillArea, ImageRepeat, type FillLike } from '@abra/core/fill';
import { displace, gaussianBlur, noise } from '@abra/core/filters';
import { Area } from '@abra/core/geometry';
import { Channels, Image, type ImageLike } from '@abra/core/image';
import { ColorStop, Gradient } from '@abra/core/gradient';
import { type AbraImageLike } from '@alakazam/mobile';
import { owning } from '@/src/lib/native/native-handles';

/** The 80s neon colors from dark to light, each at its place from 0 to 1 along the gradient. */
const NEON: [number, [number, number, number]][] = [
  [0, [20, 0, 70]],
  [0.3, [70, 30, 190]],
  [0.55, [230, 50, 180]],
  [0.8, [255, 130, 200]],
  [1, [120, 255, 240]],
];

/** Where the grid floor meets the sky, as a share of the photo's height from the top. */
const HORIZON = 0.62;
/** The sun's size as a share of the photo's shorter side, and how far it sinks below the horizon as a share of its radius. */
const SUN_RADIUS = 0.24;
const SUN_SINK = 0.85;
/** The sun's colors at its top and bottom. */
const SUN_TOP: [number, number, number] = [255, 230, 70];
const SUN_BOTTOM: [number, number, number] = [255, 50, 150];
/** The stripes cut into the sun: how many, where the first starts and how far apart they are (shares of the radius below the
 * sun's middle), and how thick the first is. Each is one step thicker than the one above. */
const SUN_CUTS = 6;
const SUN_CUT_START = 0.05;
const SUN_CUT_GAP = 0.13;
const SUN_CUT_THICKNESS = 0.02;
/** How thick the lines are, as a share of the photo's shorter side. */
const GRID_LINE = 0.004;
/** How many lines run away from the viewer on each side of the middle one, and how far apart they are at the bottom edge. */
const RAYS = 14;
const RAY_SPACING = 0.12;
/** How far apart the lines are at the horizon, as a share of their spacing at the bottom: 0 meets them all in one point, and a
 * small number only brings them close, as in the reference pictures. */
const RAY_CONVERGE = 0.15;
/** How many lines run across the floor. */
const ROWS = 9;
/** How bright a part of the photo must be, from 0 to 1, before the grid shows fully over it. Darker parts hide the grid. */
const LIGHT_REACH = 0.35;
/** How far the brightest ground lifts the grid, as a share of the photo's shorter side, and how much the photo is smoothed first. */
const GRID_BEND = 0.04;
const TERRAIN_SMOOTH = 0.02;
/** How dark the floor is, from 0 for black to 1 for untouched: at the horizon, and at the bottom edge. */
const FLOOR_DARK_NEAR = 0.5;
const FLOOR_DARK_FAR = 0.2;
/** How strongly the grid shows over the photo. */
const GRID_STRENGTH = 1;
/** How far the glow blur reaches, as a share of the photo's shorter side. */
const GLOW_BLUR = 0.012;
/** How strongly the glow shows over the colored photo. */
const GLOW_STRENGTH = 0.45;
/** How far the red and blue fringes slide apart, as a share of the photo's width. */
const FRINGE = 0.004;
/** How strong the analog video grain is. */
const GRAIN = 0.12;

/**
 * Turns `photo` into a vaporwave look in place: its brightness is stretched to use the whole range, then mapped to a gradient
 * from deep purple in the darks, through hot pink, to bright cyan in the lights. A neon perspective grid runs from the horizon
 * to the bottom edge, as a floor. A soft neon glow is added over the lights, the red and blue edges slide slightly apart like a
 * mis-aligned screen, and a light analog grain goes on top. At 0 the photo is unchanged and at 1 it is fully vaporwave. Values in
 * between fade from one to the other. `parts` says whether the striped sun and the grid floor (with its dark ground) are drawn,
 * and the color of the grid's lines as red, green and blue from 0 to 255; the color, glow, fringe and grain are always there.
 */
export const vaporwave = (
  photo: AbraImageLike,
  amount: number,
  parts: { sun: boolean; grid: boolean; gridColor: [number, number, number] },
): void =>
  owning(keep => {
    if (amount <= 0) return;
    const stops = NEON.map(([place, rgb]) => keep(new ColorStop(keep(Color.fromRgb(...rgb)), place)));
    const gradient = keep(new Gradient(stops));
    const map = keep(photo.clone());
    keep(autoTone()).apply(map);
    keep(gradientMap(gradient)).apply(map);

    const width = map.width();
    const height = map.height();
    const horizon = height * HORIZON;
    const floor = height - horizon;
    const black = keep(Color.fromRgb(0, 0, 0));
    const solid = (rgb: [number, number, number]) => keep(Fill.solid(keep(Color.fromRgb(...rgb))));
    const paint = (layer: ImageLike, fill: FillLike, points: number[][]): void =>
      void keep(fillArea(keep(Area.fromPoints(points)), fill)).apply(layer);
    const band = (layer: ImageLike, fill: FillLike, top: number, bottom: number): void =>
      paint(layer, fill, [[0, top], [width, top], [width, bottom], [0, bottom]]);

    const frame = keep(Area.fromPoints([[0, 0], [width, 0], [width, height], [0, height]]));
    // A fill whose color changes from the top of the photo to the bottom: one pixel wide, a row for each row of the photo, and
    // repeated across it. It is one fill, where a band for each step would be one for every step.
    const ramp = (colorAt: (y: number) => number[]) => {
      const pixels = new Uint8Array(height * 4);
      for (let y = 0; y < height; y++) pixels.set([...colorAt(y + 0.5), 255], y * 4);
      const column = keep(Image.newFromPixels(1, height, pixels.buffer, Channels.Rgba));
      return keep(Fill.image(column).withRepeat(ImageRepeat.Both));
    };

    const short = Math.min(width, height);
    // The sun: a circle of sunset colors, cut by dark stripes that get thicker toward the bottom, and sliced off at the horizon.
    if (parts.sun) {
      const radius = short * SUN_RADIUS;
      const sunY = horizon - radius * SUN_SINK;
      const colors = ramp(y => {
        const place = Math.min(1, Math.max(0, (y - (sunY - radius)) / (radius * 2)));
        return SUN_TOP.map((top, c) => Math.round(top + (SUN_BOTTOM[c] - top) * place));
      });
      const disc = keep(Image.newFromColor(width, height, black));
      keep(fillArea(keep(Area.circle({ x: width / 2, y: sunY }, radius)), solid([255, 255, 255]))).apply(disc);
      for (let cut = 0; cut < SUN_CUTS; cut++) {
        const top = sunY + radius * (SUN_CUT_START + cut * SUN_CUT_GAP);
        band(disc, solid([0, 0, 0]), top, top + radius * SUN_CUT_THICKNESS * (cut + 1));
      }
      band(disc, solid([0, 0, 0]), horizon, height);
      // The sun replaces the sky where the disc is white, so its colors stay solid instead of washing out to white.
      keep(keep(fillArea(frame, colors)).withMask(disc)).apply(map);
    }

    if (parts.grid) {
      // The floor is dark, so the grid glows against it: the photo is darkened from the horizon down, most at the bottom edge.
      const shade = keep(Image.newFromColor(width, height, keep(Color.fromRgb(255, 255, 255))));
      const darkening = ramp(y => {
        const level = Math.round(255 * (y < horizon ? 1 : FLOOR_DARK_NEAR + (FLOOR_DARK_FAR - FLOOR_DARK_NEAR) * Math.min(1, (y - horizon) / floor)));
        return [level, level, level];
      });
      keep(fillArea(frame, darkening)).apply(shade);
      keep(keep(blend(shade)).withMode(BlendMode.Multiply)).apply(map);

      const grid = keep(Image.newFromColor(width, height, black));
      const lineFill = solid(parts.gridColor);
      const stroke = (points: number[][]): void => paint(grid, lineFill, points);
      const thickness = Math.max(1, Math.min(width, height) * GRID_LINE);
      // Lines that run away from the viewer: each leaves the vanishing point and reaches the bottom edge at its own place.
      for (let i = -RAYS; i <= RAYS; i++) {
        const bottom = width / 2 + i * width * RAY_SPACING;
        const far = width / 2 + i * width * RAY_SPACING * RAY_CONVERGE;
        stroke([
          [far - thickness * 0.3, horizon],
          [far + thickness * 0.3, horizon],
          [bottom + thickness, height],
          [bottom - thickness, height],
        ]);
      }
      // Lines across the floor: squared depth bunches them near the horizon and spreads them toward the viewer.
      for (let row = 1; row <= ROWS; row++) {
        // One step past the last row keeps the nearest line off the bottom edge, where the bend would smear it into a band.
        const depth = (row / (ROWS + 1)) ** 2;
        const y = horizon + depth * floor;
        const half = thickness * (0.3 + depth) * 0.5;
        stroke([
          [0, y - half],
          [width, y - half],
          [width, y + half],
          [0, y + half],
        ]);
      }
      // The photo's own light rides on the grid: lines fade out where the photo is dark, so trees, people and buildings cover them
      // and the grid seems to sit on the ground behind, and they glow brighter over light ground.
      const light = keep(photo.clone());
      keep(autoTone()).apply(light);
      keep(grayscale()).apply(light);
      keep(levels(0, 255 * LIGHT_REACH)).apply(light);
      keep(gaussianBlur(Math.max(1, thickness))).apply(light);
      // The lines also bend to follow the land: the brighter the photo, the higher they ride, like a wireframe draped over terrain.
      const terrain = keep(light.clone());
      keep(gaussianBlur(Math.max(2, short * TERRAIN_SMOOTH))).apply(terrain);
      keep(displace(terrain, 0, -short * GRID_BEND)).apply(grid);
      keep(keep(blend(light)).withMode(BlendMode.Multiply)).apply(grid);
      keep(gaussianBlur(Math.max(1, thickness * 0.5))).apply(grid);
      keep(keep(keep(blend(grid)).withMode(BlendMode.Screen)).withOpacity(GRID_STRENGTH)).apply(map);
    }

    const glow = keep(map.clone());
    keep(gaussianBlur(Math.max(2, Math.min(map.width(), map.height()) * GLOW_BLUR))).apply(glow);
    keep(keep(keep(blend(glow)).withMode(BlendMode.Screen)).withOpacity(GLOW_STRENGTH)).apply(map);

    const shift = Math.max(1, map.width() * FRINGE);
    const copy = keep(map.clone());
    keep(keep(keep(blend(copy)).withOffset({ x: -shift, y: 0 })).withChannels({ red: true, green: false, blue: false })).apply(map);
    keep(keep(keep(blend(copy)).withOffset({ x: shift, y: 0 })).withChannels({ red: false, green: false, blue: true })).apply(map);

    keep(noise(GRAIN)).apply(map);
    keep(keep(blend(map)).withOpacity(amount)).apply(photo);
  });
