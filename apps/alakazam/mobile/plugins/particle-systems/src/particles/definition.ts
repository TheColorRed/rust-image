import { Impulse, PoseNoise, Schedule, SpeedResponse, type ScheduleLike, type SpeedResponseLike } from '@abra/core/animation';
import { BlendMode, blendModeFromName } from '@abra/core/blend';
import { Color } from '@abra/core/color';
import { Curve } from '@abra/core/geometry';
import { ColorStop, Gradient } from '@abra/core/gradient';
import { Emitter, ParticleSystem, type ParticleSystemLike } from '@abra/core/particles';
import { Blend, CurveRange, type BlendLike, type CurveRangeLike } from '@abra/core/sprites';
import { release } from '@alakazam/sdk';

/**
 * What a person makes in the Particle Maker, and what is saved: plain data, so it is written as JSON and read back by any project. It
 * is also independent of the picture it is used on. Places are shares of the picture's width and height (0 to 1), and sizes, speeds and
 * pulls are shares of its shorter side, so one system looks the same on a small photo and a large one. `toSystem` makes the
 * `ParticleSystem` of the simulation for a picture of a given size.
 */

/** The named shapes a curve can start from. A save of an earlier release holds the shape of a value over a life by one of these names. */
export type Over = 'Constant' | 'Fade Out' | 'Fade In' | 'Fade In & Out' | 'Grow' | 'Shrink';
export const OVERS: Over[] = ['Constant', 'Fade Out', 'Fade In', 'Fade In & Out', 'Grow', 'Shrink'];

/** A curve, as a smooth line through points `[age, multiplier]`, with the age from 0 (birth) to 1 (end of life). It is never empty. */
export type CurveDef = [number, number][];

/** A value between `min` and `max` (each particle picks its own place between them) that is multiplied by a curve over the life. */
export type RangeDef = { min: number; max: number; curve: CurveDef };

/** The shape of each name, as the multiplier at an age from 0 (birth) to 255 (end of life). */
const SHAPES: Record<Over, [number, number][]> = {
  Constant: [[0, 1]],
  'Fade Out': [[0, 1], [255, 0]],
  'Fade In': [[0, 0], [255, 1]],
  'Fade In & Out': [[0, 0], [64, 1], [191, 1], [255, 0]],
  Grow: [[0, 0.2], [255, 1]],
  Shrink: [[0, 1], [255, 0.2]],
};

export type EmitterDef =
  | { type: 'image' }
  | { type: 'circle'; x: number; y: number; radius: number }
  | { type: 'ring'; x: number; y: number; radius: number; inner: number }
  | { type: 'rect'; x: number; y: number; width: number; height: number }
  | { type: 'line'; x1: number; y1: number; x2: number; y2: number }
  | { type: 'luminance'; min: number };

export const EMITTER_TYPES: EmitterDef['type'][] = ['image', 'circle', 'ring', 'rect', 'line', 'luminance'];

/**
 * Where particles come from, as a point (shares of the picture's width and height), for an emitter that has one: the middle of a circle, a ring, a
 * rectangle or a line. The whole picture and the bright parts have no one place, so they have none.
 */
export const sourceOf = (emitter: EmitterDef): [number, number] | null => {
  switch (emitter.type) {
    case 'circle':
    case 'ring':
      return [emitter.x, emitter.y];
    case 'rect':
      return [emitter.x + emitter.width / 2, emitter.y + emitter.height / 2];
    case 'line':
      return [(emitter.x1 + emitter.x2) / 2, (emitter.y1 + emitter.y2) / 2];
    default:
      return null;
  }
};

/** Moves the emitter so its source is at `point`, keeping its size and shape. An emitter with no source is left as it is. */
export const moveSource = (emitter: EmitterDef, point: [number, number]) => {
  const [x, y] = point;
  switch (emitter.type) {
    case 'circle':
    case 'ring':
      emitter.x = x;
      emitter.y = y;
      break;
    case 'rect':
      emitter.x = x - emitter.width / 2;
      emitter.y = y - emitter.height / 2;
      break;
    case 'line': {
      const [middleX, middleY] = sourceOf(emitter)!;
      emitter.x1 += x - middleX;
      emitter.x2 += x - middleX;
      emitter.y1 += y - middleY;
      emitter.y2 += y - middleY;
      break;
    }
  }
};

/**
 * A gradient as Unity edits it: the colors it passes through and, apart from them, how strong it is along its length, each at a place from 0 (the
 * start of a life) to 1 (its end), with the strength from 0 (not there) to 255 (solid). Neither list is empty.
 */
export type GradientDef = { colors: { at: number; color: [number, number, number] }[]; alphas: { at: number; alpha: number }[] };

/** A gradient that changes nothing: white and solid all the way. */
const plainGradient = (): GradientDef => ({
  colors: [{ at: 0, color: [255, 255, 255] }, { at: 1, color: [255, 255, 255] }],
  alphas: [{ at: 0, alpha: 255 }, { at: 1, alpha: 255 }],
});

/** A gradient through these colors, evenly spaced and solid, which is what a list of start colors of an earlier release was. */
const evenly = (colors: [number, number, number][]): GradientDef => ({
  colors: colors.length > 1 ? colors.map((color, at) => ({ at: at / (colors.length - 1), color })) : [{ at: 0, color: colors[0] }, { at: 1, color: colors[0] }],
  alphas: plainGradient().alphas,
});

/** The gradients the Color Over Life choices of an earlier release made, which a save of that release holds by name, and which the starters use. */
const FADES: Record<string, () => GradientDef> = {
  'Fade Out': () => ({ ...plainGradient(), alphas: [{ at: 0, alpha: 255 }, { at: 1, alpha: 0 }] }),
  'Fade In & Out': () => ({ ...plainGradient(), alphas: [{ at: 0, alpha: 0 }, { at: 0.5, alpha: 255 }, { at: 1, alpha: 0 }] }),
  Darken: () => ({ ...plainGradient(), colors: [{ at: 0, color: [255, 255, 255] }, { at: 1, color: [90, 90, 90] }] }),
};

/** The modules that can be switched off. Emission is switched by `emission.enabled`, and the rest of a system (its size, color and speed to start with, and how it is drawn) is always on. */
export const MODULE_NAMES = [
  'shape',
  'velocity',
  'limitVelocity',
  'force',
  'colorOverLifetime',
  'colorBySpeed',
  'sizeOverLifetime',
  'sizeBySpeed',
  'rotationOverLifetime',
  'rotationBySpeed',
  'noise',
  'collision',
  'subEmitters',
  'trails',
] as const;
export type ModuleName = (typeof MODULE_NAMES)[number];

export type SystemDef = {
  emitter: EmitterDef;
  count: number;
  /** The radius, as a percent of the picture's shorter side. */
  size: RangeDef;
  /** 0 (a soft edge) to 1 (a hard one). */
  hardness: number;
  opacity: RangeDef;
  /** Degrees. */
  rotation: RangeDef;
  blend: string;
  /** The gradient each particle starts with a random color and strength from, or null for the color of the picture where it is born. */
  startColor: GradientDef | null;
  /** The color and strength a particle is multiplied by over its life, as a gradient. */
  lifetimeColor: GradientDef;
  /** Speed along x and y, each as a share of the shorter side over a whole life. */
  speedX: RangeDef;
  speedY: RangeDef;
  /**
   * A launch on top of the speeds: a speed (a share of the shorter side over a life) in a direction within `spread` degrees either side of `angle`
   * (0 is up, 90 is right), or straight out from the center of the emitter when `outward`. A speed of 0 is no launch.
   */
  launch: { angle: number; spread: number; speed: RangeDef; outward: boolean };
  /** A pull down (negative up) as a share of the shorter side. */
  gravity: number;
  /** How quickly the start speed is lost over a life: 0 keeps it, and the higher it is the sooner a particle slows to a stop. */
  drag: number;
  /** A constant push sideways and up or down (negative up), as a share of the shorter side, like a wind. */
  windX: number;
  windY: number;
  /** Degrees spun around the center over a life, and the push away from it (negative toward) as a share of the shorter side. */
  orbit: RangeDef;
  push: RangeDef;
  /**
   * Smooth noise that pushes particles about, as in Unity's Noise: how far it pushes as a share of the shorter side, how many swirls fit across the
   * shorter side (`frequency`), how many swirls the noise slides across the picture over a life in x and in y, and how many layers of finer
   * detail are added (`octaves`, at least 1). 0 strength is none.
   */
  noise: {
    strength: RangeDef;
    frequency: number;
    scrollX: number;
    scrollY: number;
    octaves: number;
    /** Whether the strength is a share of one swirl, so bigger swirls move particles farther (Unity's damping). */
    damping: boolean;
    /** How much the noise changes the size, as a share of it, and the rotation, in degrees. */
    sizeAmount: number;
    rotationAmount: number;
  };
  /** Whether particles bounce off the sides of the picture instead of leaving it (Unity's Collision). */
  bounce: boolean;
  /** How far in from each side of the picture the walls they bounce off are, as a share of that side (0 to 0.4). */
  bounceInset: number;
  /** The size is multiplied by `slow` when a particle is still and by `fast` at `speed` (a share of the shorter side over a life) and above. */
  sizeBySpeed: { slow: number; fast: number; speed: number };
  /** The rotation is turned by `slow` degrees when a particle is still and by `fast` at `sizeBySpeed.speed` and above, which is the speed all three by-speed modules share. */
  rotationBySpeed: { slow: number; fast: number };
  /** The color is multiplied by `slow` when a particle is still and by `fast` at `sizeBySpeed.speed` and above, each `[r, g, b]`. White changes nothing. */
  colorBySpeed: { slow: [number, number, number]; fast: [number, number, number] };
  /**
   * Which of the modules (the groups of the inspector with a checkbox, as in Unity's particle system) the person has switched on or off. A module
   * that is not here is on when it holds something other than a blank system's values, so a system that was made before modules, and a module that
   * has been set, are on without anyone having ticked them. A module that is off keeps its values but they are not used.
   */
  modules: Partial<Record<ModuleName, boolean>>;
  /** How much longer a particle is for each share of the shorter side over a life it goes, turned the way it goes. 0 is none. */
  stretch: number;
  /** When particles are born and how long they live, in seconds. Off, the particles are the `count` and the life is the preview's. */
  emission: { enabled: boolean; duration: number; looping: boolean; rate: number; burst: number; lifetime: RangeDef };
  /** How far each particle's own age may differ from the preview's, so they are not all one age. */
  ageSpread: number;
  /** A streak behind each particle, as a share of its life (0 for none). */
  trail: { length: number; fade: number; hardness: number };
  /** Systems born from each particle once it reaches an age (0 to 1). */
  children: { age: number; system: SystemDef }[];
};

/** What a save holds. The version lets a later release read a system saved by this one. */
export type SavedSystem = { version: 1; name: string; system: SystemDef };

/** The curve of a named shape. */
const curveOf = (over: Over): CurveDef => SHAPES[over].map(([age, multiplier]) => [age / 255, multiplier]);

/** The named shapes as curves, which the curve editor offers to start from. */
export const CURVE_PRESETS = OVERS.map(name => ({ label: name, value: curveOf(name) }));

const flat = (min: number, max = min, over: Over = 'Constant'): RangeDef => ({ min, max, curve: curveOf(over) });

/** A blank system: a soft, slow cloud of white dots over the whole picture. */
export const blankSystem = (): SystemDef => ({
  emitter: { type: 'image' },
  count: 120,
  size: flat(0.4, 1.2),
  hardness: 0,
  opacity: flat(0.5, 1),
  rotation: flat(0),
  blend: 'Normal',
  startColor: plainGradient(),
  lifetimeColor: plainGradient(),
  speedX: flat(0),
  speedY: flat(0),
  launch: { angle: 0, spread: 30, speed: flat(0), outward: false },
  gravity: 0,
  drag: 0,
  windX: 0,
  windY: 0,
  orbit: flat(0),
  push: flat(0),
  noise: { strength: flat(0), frequency: 10, scrollX: 0, scrollY: 0, octaves: 1, damping: false, sizeAmount: 0, rotationAmount: 0 },
  bounce: false,
  bounceInset: 0,
  sizeBySpeed: { slow: 1, fast: 1, speed: 0.5 },
  rotationBySpeed: { slow: 0, fast: 0 },
  colorBySpeed: { slow: [255, 255, 255], fast: [255, 255, 255] },
  modules: {},
  stretch: 0,
  emission: { enabled: false, duration: 3, looping: true, rate: 40, burst: 0, lifetime: flat(1, 2) },
  ageSpread: 0,
  trail: { length: 0, fade: 1, hardness: 0.5 },
  children: [],
});

/** The systems a new one can start from. A new starting point is a line here. */
export const STARTERS: Record<string, () => SystemDef> = {
  Blank: blankSystem,
  Fire: () => ({
    ...blankSystem(),
    emitter: { type: 'line', x1: 0.3, y1: 0.95, x2: 0.7, y2: 0.95 },
    count: 400,
    size: flat(0.5, 1.6, 'Shrink'),
    opacity: flat(0.6, 1),
    lifetimeColor: FADES['Fade Out'](),
    blend: 'Additive',
    startColor: evenly([[255, 240, 160], [255, 140, 30], [220, 40, 10]]),
    speedX: flat(-0.05, 0.05),
    speedY: flat(-0.5, -0.2),
    gravity: -0.1,
    noise: { strength: flat(0.02, 0.05), frequency: 10, scrollX: 0, scrollY: 0, octaves: 1, damping: false, sizeAmount: 0, rotationAmount: 0 },
    ageSpread: 0.4,
  }),
  Snow: () => ({
    ...blankSystem(),
    count: 250,
    size: flat(0.2, 0.7),
    hardness: 0.3,
    opacity: flat(0.6, 1),
    startColor: evenly([[255, 255, 255], [220, 235, 255]]),
    speedX: flat(-0.05, 0.05),
    speedY: flat(0.1, 0.3),
    noise: { strength: flat(0.02, 0.04), frequency: 7, scrollX: 0, scrollY: 0, octaves: 1, damping: false, sizeAmount: 0, rotationAmount: 0 },
    ageSpread: 1,
  }),
  Sparks: () => ({
    ...blankSystem(),
    emitter: { type: 'circle', x: 0.5, y: 0.5, radius: 0.03 },
    count: 80,
    size: flat(0.15, 0.4, 'Shrink'),
    hardness: 0.5,
    blend: 'Additive',
    startColor: evenly([[255, 255, 255], [255, 220, 120]]),
    lifetimeColor: FADES['Fade Out'](),
    launch: { angle: 0, spread: 180, speed: flat(0.1, 0.6), outward: true },
    gravity: 0.25,
    trail: { length: 0.4, fade: 1, hardness: 0.5 },
    ageSpread: 0.2,
  }),
  Smoke: () => ({
    ...blankSystem(),
    emitter: { type: 'circle', x: 0.5, y: 0.9, radius: 0.04 },
    count: 60,
    size: flat(2, 4, 'Grow'),
    opacity: flat(0.15, 0.35),
    lifetimeColor: FADES['Fade In & Out'](),
    startColor: evenly([[90, 90, 90], [150, 150, 150]]),
    speedX: flat(-0.05, 0.05),
    speedY: flat(-0.35, -0.15),
    noise: { strength: flat(0.04, 0.08), frequency: 5, scrollX: 0, scrollY: 0, octaves: 1, damping: false, sizeAmount: 0, rotationAmount: 0 },
    ageSpread: 0.6,
  }),
};

/** The colors to choose from, by name, so a color is picked from a list and not typed. A new color is a line here. */
export const SWATCHES: Record<string, [number, number, number]> = {
  White: [255, 255, 255],
  Cream: [255, 240, 200],
  Yellow: [255, 214, 10],
  Gold: [255, 190, 60],
  Orange: [255, 140, 30],
  Ember: [220, 60, 20],
  Red: [235, 40, 40],
  Pink: [255, 120, 190],
  Purple: [160, 90, 240],
  Blue: [50, 120, 255],
  Ice: [150, 215, 255],
  Cyan: [60, 220, 230],
  Green: [80, 210, 110],
  Gray: [128, 128, 128],
  Smoke: [90, 90, 90],
  Black: [0, 0, 0],
};

/** Where a new emitter of each kind starts, which is also what a saved emitter falls back to for a number it lacks. */
export const EMITTER_STARTS: Record<EmitterDef['type'], EmitterDef> = {
  image: { type: 'image' },
  circle: { type: 'circle', x: 0.5, y: 0.5, radius: 0.15 },
  ring: { type: 'ring', x: 0.5, y: 0.5, radius: 0.2, inner: 0.1 },
  rect: { type: 'rect', x: 0.25, y: 0.25, width: 0.5, height: 0.5 },
  line: { type: 'line', x1: 0.2, y1: 0.9, x2: 0.8, y2: 0.9 },
  luminance: { type: 'luminance', min: 0.7 },
};

/** The most a sub-emitter can be inside another, so a system cannot grow without end. */
export const MAX_CHILD_DEPTH = 3;

const isObject = (value: unknown): value is Record<string, unknown> => typeof value === 'object' && value !== null && !Array.isArray(value);

/** A real number held between `low` and `high`, or `fallback` when the value is not one: missing, NaN, infinite, or not a number at all. */
export const finite = (value: unknown, fallback: number, low = -Infinity, high = Infinity) =>
  typeof value === 'number' && Number.isFinite(value) ? Math.min(Math.max(value, low), high) : fallback;

/** A curve with every point a real age (0 to 1) and multiplier (0 to 1) and in order, or `fallback` when it has none. */
const cleanCurve = (value: unknown, fallback: CurveDef): CurveDef => {
  const points = (Array.isArray(value) ? value : [])
    .filter((point): point is unknown[] => Array.isArray(point) && point.length >= 2)
    .map(point => [finite(point[0], 0, 0, 1), finite(point[1], 1, 0, 1)] as [number, number])
    .sort((a, b) => a[0] - b[0]);
  return points.length ? points : fallback.map(point => [...point] as [number, number]);
};

const cleanRange = (value: unknown, fallback: RangeDef): RangeDef => {
  const given = isObject(value) ? value : {};
  return {
    min: finite(given.min, fallback.min),
    max: finite(given.max, fallback.max),
    curve: cleanCurve(given.curve, OVERS.includes(given.over as Over) ? curveOf(given.over as Over) : fallback.curve),
  };
};

/** An emitter of a kind it knows, with every number it needs: one that lacks a number (an older save, a hand-edited file) gets the start's. */
const cleanEmitter = (value: unknown): EmitterDef => {
  const given = isObject(value) ? value : {};
  const type = EMITTER_TYPES.includes(given.type as EmitterDef['type']) ? (given.type as EmitterDef['type']) : 'image';
  return Object.fromEntries(
    Object.entries(EMITTER_STARTS[type]).map(([key, start]) => [key, key === 'type' ? type : finite(given[key], start as number)]),
  ) as EmitterDef;
};

/** The colors of a system: none (the picture's own), or one to four of `[r, g, b]` with each channel a whole number from 0 to 255. */
const cleanColors = (value: unknown, fallback: [number, number, number][]): [number, number, number][] | null => {
  if (value === null) return null;
  if (!Array.isArray(value)) return fallback;
  const colors = value
    .filter((color): color is unknown[] => Array.isArray(color) && color.length >= 3)
    .slice(0, 4)
    .map(color => [0, 1, 2].map(at => Math.round(finite(color[at], 255, 0, 255))) as [number, number, number]);
  return colors.length ? colors : fallback;
};

/** The colors of an earlier release's list (one to four of `[r, g, b]`, each channel a whole number from 0 to 255) as a gradient, or nothing when there are none. */
const cleanedEvenly = (colors: unknown[]) => {
  const clean = cleanColors(colors, []);
  return clean?.length ? evenly(clean) : undefined;
};

/** A gradient with every stop a real place and value in range and each list in order and not empty, or `fallback` when it is not one. */
const cleanGradient = (value: unknown, fallback: GradientDef): GradientDef => {
  const given = isObject(value) ? value : {};
  const stops = (list: unknown) => (Array.isArray(list) ? list.filter(isObject) : []);
  const colors = stops(given.colors)
    .map(stop => ({ at: finite(stop.at, 0, 0, 1), color: cleanColors([stop.color], [[255, 255, 255]])![0] }))
    .sort((a, b) => a.at - b.at);
  const alphas = stops(given.alphas)
    .map(stop => ({ at: finite(stop.at, 0, 0, 1), alpha: Math.round(finite(stop.alpha, 255, 0, 255)) }))
    .sort((a, b) => a.at - b.at);
  return colors.length && alphas.length ? { colors, alphas } : fallback;
};

/**
 * A system with every setting a real value of the right kind, from anything that was read from a file or typed in. What is missing, NaN, out of its
 * range or of the wrong kind is put right one setting at a time, so one bad number does not turn the picture blank or throw away the rest, and
 * sub-emitters are put right in the same way.
 */
export const cleanSystem = (value: unknown, depth = 0): SystemDef => {
  const base = blankSystem();
  const given = isObject(value) ? value : {};
  const noise = isObject(given.noise) ? given.noise : {};
  const launch = isObject(given.launch) ? given.launch : {};
  const sizeBySpeed = isObject(given.sizeBySpeed) ? given.sizeBySpeed : {};
  const rotationBySpeed = isObject(given.rotationBySpeed) ? given.rotationBySpeed : {};
  const colorBySpeed = isObject(given.colorBySpeed) ? given.colorBySpeed : {};
  const modules = isObject(given.modules) ? given.modules : {};
  const emission = isObject(given.emission) ? given.emission : {};
  const trail = isObject(given.trail) ? given.trail : {};
  const opacity = cleanRange(given.opacity, base.opacity);
  // A save of an earlier release holds a list of colors, or null for the picture's.
  const startColor =
    given.startColor === null || (given.startColor === undefined && given.colors === null)
      ? null
      : cleanGradient(given.startColor ?? (Array.isArray(given.colors) ? cleanedEvenly(given.colors) : undefined), base.startColor!);
  // A save of an earlier release holds the Color Over Life choice by name, and the opacity over a life as a shape on the opacity. Both are the
  // gradient now, whose strength follows the life.
  const lifetimeColor = cleanGradient(
    given.lifetimeColor ?? (typeof given.fade === 'string' ? FADES[given.fade]?.() : undefined),
    base.lifetimeColor,
  );
  const opacityOver = isObject(given.opacity) ? given.opacity.over : undefined;
  if (OVERS.includes(opacityOver as Over) && opacityOver !== 'Constant' && lifetimeColor.alphas.every(stop => stop.alpha === 255)) {
    lifetimeColor.alphas = SHAPES[opacityOver as Over].map(([age, multiplier]) => ({ at: age / 255, alpha: Math.round(multiplier * 255) }));
  }
  // The opacity has no curve of its own: it is the gradient's strength.
  opacity.curve = curveOf('Constant');
  return {
    emitter: cleanEmitter(given.emitter),
    count: Math.round(finite(given.count, base.count, 0, 5000)),
    size: cleanRange(given.size, base.size),
    hardness: finite(given.hardness, base.hardness, 0, 1),
    opacity,
    rotation: cleanRange(given.rotation, base.rotation),
    blend: typeof given.blend === 'string' ? given.blend : base.blend,
    startColor,
    lifetimeColor,
    speedX: cleanRange(given.speedX, base.speedX),
    speedY: cleanRange(given.speedY, base.speedY),
    launch: {
      angle: finite(launch.angle, base.launch.angle, -360, 360),
      spread: finite(launch.spread, base.launch.spread, 0, 180),
      speed: cleanRange(launch.speed, base.launch.speed),
      outward: typeof launch.outward === 'boolean' ? launch.outward : base.launch.outward,
    },
    gravity: finite(given.gravity, base.gravity),
    drag: finite(given.drag, base.drag, 0, 20),
    windX: finite(given.windX, base.windX),
    windY: finite(given.windY, base.windY),
    orbit: cleanRange(given.orbit, base.orbit),
    push: cleanRange(given.push, base.push),
    noise: {
      strength: cleanRange(noise.strength, base.noise.strength),
      frequency: finite(noise.frequency, base.noise.frequency, 0.1, 200),
      scrollX: finite(noise.scrollX, base.noise.scrollX),
      scrollY: finite(noise.scrollY, base.noise.scrollY),
      octaves: Math.round(finite(noise.octaves, base.noise.octaves, 1, 4)),
      damping: typeof noise.damping === 'boolean' ? noise.damping : base.noise.damping,
      sizeAmount: finite(noise.sizeAmount, base.noise.sizeAmount, 0, 5),
      rotationAmount: finite(noise.rotationAmount, base.noise.rotationAmount, 0, 360),
    },
    bounce: typeof given.bounce === 'boolean' ? given.bounce : base.bounce,
    bounceInset: finite(given.bounceInset, base.bounceInset, 0, 0.4),
    rotationBySpeed: {
      slow: finite(rotationBySpeed.slow, base.rotationBySpeed.slow, -720, 720),
      fast: finite(rotationBySpeed.fast, base.rotationBySpeed.fast, -720, 720),
    },
    colorBySpeed: {
      slow: cleanColors([colorBySpeed.slow], [base.colorBySpeed.slow])![0],
      fast: cleanColors([colorBySpeed.fast], [base.colorBySpeed.fast])![0],
    },
    modules: Object.fromEntries(MODULE_NAMES.filter(name => typeof modules[name] === 'boolean').map(name => [name, modules[name]])),
    sizeBySpeed: {
      slow: finite(sizeBySpeed.slow, base.sizeBySpeed.slow, 0, 20),
      fast: finite(sizeBySpeed.fast, base.sizeBySpeed.fast, 0, 20),
      speed: finite(sizeBySpeed.speed, base.sizeBySpeed.speed, 0.01, 20),
    },
    stretch: finite(given.stretch, base.stretch, 0, 50),
    emission: {
      enabled: typeof emission.enabled === 'boolean' ? emission.enabled : base.emission.enabled,
      duration: finite(emission.duration, base.emission.duration, 0.1, 60),
      looping: typeof emission.looping === 'boolean' ? emission.looping : base.emission.looping,
      rate: finite(emission.rate, base.emission.rate, 0, 2000),
      burst: Math.round(finite(emission.burst, base.emission.burst, 0, 5000)),
      lifetime: cleanRange(emission.lifetime, base.emission.lifetime),
    },
    ageSpread: finite(given.ageSpread, base.ageSpread, 0, 1),
    trail: {
      length: finite(trail.length, base.trail.length, 0),
      fade: finite(trail.fade, base.trail.fade, 0, 1),
      hardness: finite(trail.hardness, base.trail.hardness, 0, 1),
    },
    children:
      Array.isArray(given.children) && depth < MAX_CHILD_DEPTH
        ? given.children.map(child => {
            const entry = isObject(child) ? child : {};
            return { age: finite(entry.age, 0.5, 0, 1), system: cleanSystem(entry.system, depth + 1) };
          })
        : [],
  };
};

/** A system that came from a file, or nothing when the file is not one: what it holds is put right (see `cleanSystem`), so a save from another release or one that was edited by hand cannot break the maker. */
export const readSystem = (text: string): SavedSystem | null => {
  let parsed: unknown;
  try {
    parsed = JSON.parse(text);
  } catch {
    return null;
  }
  if (!isObject(parsed) || typeof parsed.name !== 'string' || !isObject(parsed.system)) return null;
  return { version: 1, name: parsed.name, system: cleanSystem(parsed.system) };
};

/**
 * The system as it is shown when there is no picture under it: a color taken from the picture is white, and a place that is the bright parts of
 * the picture is the whole picture, since there are no pixels to read. Nothing is changed in the saved system.
 */
export const withoutPicture = (system: SystemDef): SystemDef => ({
  ...system,
  startColor: system.startColor ?? plainGradient(),
  emitter: system.emitter.type === 'luminance' ? { type: 'image' } : system.emitter,
  children: system.children.map(child => ({ ...child, system: withoutPicture(child.system) })),
});

/** What switching a module off puts back to a blank system's values, which is how its values are left out. */
const RESET: Record<ModuleName, (system: SystemDef, blank: SystemDef) => void> = {
  shape: (system, blank) => void (system.emitter = blank.emitter),
  velocity: (system, blank) => void Object.assign(system, { orbit: blank.orbit, push: blank.push }),
  limitVelocity: (system, blank) => void (system.drag = blank.drag),
  force: (system, blank) => void Object.assign(system, { windX: blank.windX, windY: blank.windY }),
  colorOverLifetime: (system, blank) => {
    system.lifetimeColor = blank.lifetimeColor;
  },
  colorBySpeed: (system, blank) => void (system.colorBySpeed = blank.colorBySpeed),
  sizeOverLifetime: (system, blank) => void (system.size = { ...system.size, curve: blank.size.curve }),
  sizeBySpeed: (system, blank) => void Object.assign(system.sizeBySpeed, { slow: blank.sizeBySpeed.slow, fast: blank.sizeBySpeed.fast }),
  rotationOverLifetime: (system, blank) => void (system.rotation = blank.rotation),
  rotationBySpeed: (system, blank) => void (system.rotationBySpeed = blank.rotationBySpeed),
  noise: (system, blank) => void (system.noise = blank.noise),
  collision: (system, blank) => void Object.assign(system, { bounce: blank.bounce, bounceInset: blank.bounceInset }),
  subEmitters: (system, blank) => void (system.children = blank.children),
  trails: (system, blank) => void (system.trail = blank.trail),
};

/** Whether a module is on: as the person set it, or when they have not, whether it holds anything but a blank system's values. */
export const moduleOn = (system: SystemDef, name: ModuleName): boolean => {
  const set = system.modules[name];
  if (set !== undefined) return set;
  const reset: SystemDef = JSON.parse(JSON.stringify(system));
  RESET[name](reset, blankSystem());
  return JSON.stringify(reset) !== JSON.stringify(system);
};

/** The system with every module that is off put back to a blank system's values, and so left out. Sub-emitters are put right in the same way. */
export const withModules = (system: SystemDef): SystemDef => {
  const used: SystemDef = JSON.parse(JSON.stringify(system));
  const blank = blankSystem();
  // What each module is is worked out before any is reset, since a module that is set is on and one that is reset is not.
  for (const name of MODULE_NAMES) if (!moduleOn(system, name)) RESET[name](used, blank);
  used.children = used.children.map(child => ({ ...child, system: withModules(child.system) }));
  return used;
};

/**
 * The system of the library that a definition makes, how it is blended with the picture, and how many seconds its life lasts when it makes particles over
 * time (0 when it does not). The caller lets go of the natives with `dispose`.
 */
export type MadeSystem = { system: ParticleSystemLike; blend: BlendLike; duration: number; dispose: () => void };

/** How many pieces the speed range is cut into in the table of colors by speed (the table has one more color than this). */
const SPEED_COLOR_STEPS = 256;

/** How particles are blended with what is under them: the name of any blend mode of the library, or `Additive`, which adds light. A name that is not a mode is `Normal`. */
const blendOf = (name: string): BlendLike => (name === 'Additive' ? Blend.additive() : Blend.mode(blendModeFromName(name) ?? BlendMode.Normal));

/** The gradient as one list of stops with a color and a strength each, at every place either of its lists has a stop. */
const gradientStops = (gradient: GradientDef) => {
  /** The value of a list at `at`, mixed between the stops around it. */
  const mixed = <T extends { at: number }>(list: T[], at: number, read: (stop: T) => number[]) => {
    const next = list.findIndex(stop => stop.at >= at);
    if (next === -1) return read(list[list.length - 1]);
    if (next === 0) return read(list[0]);
    const [before, after] = [list[next - 1], list[next]];
    const along = after.at > before.at ? (at - before.at) / (after.at - before.at) : 0;
    const to = read(after);
    return read(before).map((value, index) => value + (to[index] - value) * along);
  };
  const [colors, alphas] = [[...gradient.colors].sort((a, b) => a.at - b.at), [...gradient.alphas].sort((a, b) => a.at - b.at)];
  return [...new Set([...colors, ...alphas].map(stop => stop.at))]
    .sort((a, b) => a - b)
    .map(at => ({ at, rgb: mixed(colors, at, stop => stop.color).map(Math.round), alpha: Math.round(mixed(alphas, at, stop => [stop.alpha])[0]) }));
};

/** The stops of a gradient, or nothing for one that changes nothing: white and solid all the way. */
const lifetimeStops = (gradient: GradientDef) =>
  gradient.colors.every(stop => stop.color.every(channel => channel === 255)) && gradient.alphas.every(stop => stop.alpha === 255)
    ? null
    : gradientStops(gradient);

const SCALES = (range: RangeDef, scale: number): RangeDef => ({ ...range, min: range.min * scale, max: range.max * scale });

/**
 * The system of the library for a picture `width` x `height` pixels, at the given variation (seed) and how far along its life it is (0 to 1). The library
 * works out where every particle is born and what it picks; this only says what the definition asks for, in pixels. The caller lets go of what it made
 * with `dispose` once the system has been drawn.
 */
export const toSystem = (def: SystemDef, width: number, height: number, seed: number, progress: number, loop = false): MadeSystem => {
  // Whatever made the system (a file, a slider, a bug), the particles are made from one that is good, so a bad number cannot make the picture blank.
  def = cleanSystem(def);
  const natives: object[] = [];
  const keep = <T extends object>(native: T): T => {
    natives.push(native);
    return native;
  };
  const shorter = Math.min(width, height);

  /** A value that follows a curve over a life is a pair of curves; a flat one is just its two numbers. */
  const rangeOf = (range: RangeDef): CurveRangeLike => {
    if (range.curve.every(([, multiplier]) => multiplier === 1)) return keep(CurveRange.fromPair(range.min, range.max));
    const curve = (value: number) => keep(new Curve(range.curve.map(([age, multiplier]) => ({ x: age * 255, y: value * multiplier }))));
    return keep(new CurveRange(curve(range.min), curve(range.max)));
  };

  const place = (emitter: EmitterDef) => {
    switch (emitter.type) {
      case 'circle':
        return Emitter.Circle.new({ center: [emitter.x * width, emitter.y * height], radius: emitter.radius * shorter });
      case 'ring':
        return Emitter.Ring.new({ center: [emitter.x * width, emitter.y * height], radius: emitter.radius * shorter, innerRadius: emitter.inner * shorter });
      case 'rect':
        return Emitter.Rect.new({ left: emitter.x * width, top: emitter.y * height, width: emitter.width * width, height: emitter.height * height });
      case 'line':
        return Emitter.Line.new({ from: [emitter.x1 * width, emitter.y1 * height], to: [emitter.x2 * width, emitter.y2 * height] });
      case 'luminance':
        return Emitter.Luminance.new({ min: emitter.min });
      default:
        return Emitter.Image.new();
    }
  };

  const make = (given: SystemDef, root: boolean): ParticleSystemLike => {
    const system = withModules(given);
    const gradientOf = (stops: ReturnType<typeof gradientStops>) =>
      keep(new Gradient(stops.map(stop => keep(new ColorStop(keep(Color.fromRgba(stop.rgb[0], stop.rgb[1], stop.rgb[2], stop.alpha)), stop.at)))));
    const lifetime = lifetimeStops(system.lifetimeColor);

    // Each setting is a step of the library's system, which keeps the one before it until `dispose`.
    let made: ParticleSystemLike = keep(new ParticleSystem());
    const set = (change: (made: ParticleSystemLike) => ParticleSystemLike) => void (made = keep(change(made)));
    set(made => made.withEmitter(root ? place(system.emitter) : Emitter.Image.new()));
    set(made => made.withSeed(BigInt(Math.round(finite(seed, 1)))));
    set(made => made.withCount(Math.max(0, Math.round(system.count))));
    set(made => made.withSize(rangeOf(SCALES(system.size, shorter / 100))));
    set(made => made.withHardness(system.hardness));
    set(made => made.withOpacity(rangeOf(system.opacity)));
    set(made => made.withRotation(rangeOf(system.rotation)));
    if (system.startColor) set(made => made.withStartColor(gradientOf(gradientStops(system.startColor!))));
    if (lifetime) set(made => made.withColorOverLifetime(gradientOf(lifetime)));
    set(made => made.withStartSpeed(rangeOf(SCALES(system.speedX, shorter)), rangeOf(SCALES(system.speedY, shorter))));
    if (system.launch.speed.min !== 0 || system.launch.speed.max !== 0) {
      const impulse = keep(keep(new Impulse(system.launch.angle, system.launch.spread, rangeOf(SCALES(system.launch.speed, shorter)))).withOutward(system.launch.outward));
      set(made => made.withLaunch(impulse));
    }
    set(made => made.withGravity(system.gravity * shorter));
    set(made => made.withDrag(system.drag));
    set(made => made.withWind(system.windX * shorter, system.windY * shorter));
    set(made => made.withOrbitalVelocity(rangeOf(system.orbit)));
    set(made => made.withRadialVelocity(rangeOf(SCALES(system.push, shorter))));
    const swirl = Math.max(1, shorter / system.noise.frequency);
    if (system.noise.strength.min !== 0 || system.noise.strength.max !== 0) {
      set(made => made.withNoise(rangeOf(SCALES(system.noise.strength, shorter)), swirl));
      set(made => made.withNoiseOctaves(system.noise.octaves));
      set(made => made.withNoiseScroll(system.noise.scrollX, system.noise.scrollY));
      set(made => made.withNoiseDamping(system.noise.damping));
    }
    if (system.noise.sizeAmount > 0 || system.noise.rotationAmount > 0) {
      const noise = keep(new PoseNoise(system.noise.sizeAmount, system.noise.rotationAmount, swirl, BigInt(Math.round(finite(seed, 1)))));
      set(made => made.withPoseNoise(noise));
    }
    if (system.bounce) {
      const [x, y] = [system.bounceInset * width, system.bounceInset * height];
      set(made => made.withBounds(x, y, width - x, height - y));
    }
    const { sizeBySpeed, rotationBySpeed, colorBySpeed } = system;
    const changesColor = colorBySpeed.slow.some(channel => channel !== 255) || colorBySpeed.fast.some(channel => channel !== 255);
    if (sizeBySpeed.slow !== 1 || sizeBySpeed.fast !== 1 || rotationBySpeed.slow !== 0 || rotationBySpeed.fast !== 0 || changesColor) {
      // Each part is its own step of the response, and a part that changes nothing is left out.
      let response: SpeedResponseLike = keep(new SpeedResponse(0, sizeBySpeed.speed * shorter));
      const step = (change: (response: SpeedResponseLike) => SpeedResponseLike) => void (response = keep(change(response)));
      if (sizeBySpeed.slow !== 1 || sizeBySpeed.fast !== 1) {
        const curve = keep(new Curve([{ x: 0, y: sizeBySpeed.slow }, { x: 255, y: sizeBySpeed.fast }]));
        step(response => response.withSize(curve));
      }
      if (rotationBySpeed.slow !== 0 || rotationBySpeed.fast !== 0) {
        const curve = keep(new Curve([{ x: 0, y: rotationBySpeed.slow }, { x: 255, y: rotationBySpeed.fast }]));
        step(response => response.withRotation(curve));
      }
      if (changesColor) {
        // The color at each speed from the slowest to the fastest: a mix of the slow color and the fast one.
        const colors = Array.from({ length: SPEED_COLOR_STEPS + 1 }, (_, at) => {
          const mix = at / SPEED_COLOR_STEPS;
          return [...colorBySpeed.slow.map((channel, index) => channel + (colorBySpeed.fast[index] - channel) * mix), 255];
        });
        step(response => response.withColors(colors.flat()));
      }
      set(made => made.withSpeedResponse(response));
    }
    if (system.stretch > 0) set(made => made.withVelocityStretch(system.stretch / shorter));
    if (root && system.emission.enabled) {
      const { duration, looping, rate, burst, lifetime: lives } = system.emission;
      let schedule: ScheduleLike = keep(new Schedule(Math.max(duration, 0.01)));
      const step = (change: (schedule: ScheduleLike) => ScheduleLike) => void (schedule = keep(change(schedule)));
      step(schedule => schedule.withLooping(looping));
      step(schedule => schedule.withRate(rate));
      if (burst > 0) step(schedule => schedule.withBurst(0, burst));
      step(schedule => schedule.withLifetime(Math.min(lives.min, lives.max), Math.max(lives.min, lives.max)));
      set(made => made.withEmission(schedule));
    }
    set(made => made.withAgeSpread(system.ageSpread));
    set(made => made.withTrail(system.trail.length));
    set(made => made.withTrailFade(system.trail.fade));
    set(made => made.withTrailHardness(system.trail.hardness));
    set(made => made.withProgress(finite(progress, 0.5, 0, 1)));
    set(made => made.withLooping(loop));
    for (const child of system.children) {
      const born = make(child.system, false);
      set(made => made.withSubEmitter(child.age, born));
    }
    return made;
  };

  const root = withModules(def);
  const blend = blendOf(root.blend);
  natives.push(blend);
  return {
    system: make(def, true),
    blend,
    duration: root.emission.enabled ? root.emission.duration : 0,
    dispose: () => release(...natives),
  };
};
