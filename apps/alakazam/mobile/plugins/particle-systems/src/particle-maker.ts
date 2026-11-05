import { Canvas, type CanvasLike, type LayerLike } from '@abra/core/canvas';
import { Color } from '@abra/core/color';
import { Image, type ImageLike } from '@abra/core/image';
import type { SpritesLike } from '@abra/core/sprites';
import { app, inspector, release } from '@alakazam/sdk';
import {
  EMITTER_STARTS,
  EMITTER_TYPES,
  MAX_CHILD_DEPTH,
  CURVE_PRESETS,
  STARTERS,
  SWATCHES,
  blankSystem,
  finite,
  moduleOn,
  moveSource,
  readSystem,
  sourceOf,
  toSystem,
  withoutPicture,
  type EmitterDef,
  type MadeSystem,
  type ModuleName,
  type RangeDef,
  type SavedSystem,
  type SystemDef,
} from './particles/definition';
import { particleSprites, prepareParticles, type PreparedParticles } from './particles/system';

/**
 * The Particle Maker: makes particle systems, keeps each as a save of this plugin (`app.saves`), and shows the one being made playing live,
 * over a plain backdrop or over nothing. A system is not made for a picture: it is made to be used on any picture later, so the preview has no
 * picture of its own.
 */

/** A system and the file it is kept in. */
type Entry = { file: string; saved: SavedSystem };

let entries: Entry[] = [];
let current: Entry | null = null;
/** Which sub-emitter is being edited: the place of each child to go down through, from the system itself. Nothing is the system itself. */
let editing: number[] = [];
let loaded = false;
/** What is written for the current system, to tell when it changed. */
let written = '';

/** The size of the preview, which the places of a system are shares of. */
const PREVIEW: [number, number] = [1024, 768];
/** What the system is drawn over, by name. A new backdrop is a line here. */
const BACKDROPS: Record<string, [number, number, number]> = {
  Night: [10, 12, 28],
  Dusk: [70, 45, 90],
  Day: [120, 170, 230],
  Gray: [128, 128, 128],
  Black: [0, 0, 0],
  White: [255, 255, 255],
};
/** The backdrop that is nothing: the particles are shown on their own. */
const NO_BACKDROP = 'None';
/** The backdrop that is a color the person picks, in the color picker. */
const CUSTOM_BACKDROP = 'Custom';
let backdrop = 'Night';
/** The color of the Custom backdrop. */
let customBackdrop: [number, number, number] = [40, 40, 60];
/** Whether the preview plays, going round and round, or stays at one moment of the life. */
let playing = true;
/** How many seconds a play takes to go once through a particle's life. */
let cycle = 3;
/** How far along its life the system is when the preview is not playing, as a percent. */
let age = 50;
/** The variation (seed) of the scatter in the preview. */
let variation = 1;
/** When the play began, so its place in the cycle is how long ago that was. */
let startedAt = Date.now();

let canvas: CanvasLike | null = null;
/** The layer of `canvas` that each frame of the live preview is put on, in place. */
let layer: LayerLike | null = null;
/** The backdrop the canvas was made with. */
let builtFrom = '';
/**
 * The system as the library makes it, and what does not change as it goes through its life already worked out, kept for as long as the system and
 * what it is drawn over stay the same, so a frame only has to work out where each particle is at its age.
 */
let compiled: { key: string; made: MadeSystem; prepared: PreparedParticles } | null = null;
let timer: ReturnType<typeof setTimeout> | null = null;
/** Whether a tap on the preview moves the source. It starts with the first preview and stops when the workspace closes. */
let picking = false;
const backdrops = new Map<string, ImageLike>();

/** The named colors the color picker shows for a particle's color. */
const SWATCH_LIST = Object.entries(SWATCHES).map(([label, rgb]) => ({ label, rgb }));

const BLENDS = ['Normal', 'Additive', 'Screen', 'Overlay', 'Multiply', 'Lighten'];
const EMITTER_NAMES: Record<EmitterDef['type'], string> = {
  image: 'Whole picture',
  circle: 'Circle',
  ring: 'Ring',
  rect: 'Rectangle',
  line: 'Line',
  luminance: 'Bright parts',
};
const slugOf = (name: string) =>
  name
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, '-')
    .replace(/^-|-$/g, '') || 'system';

/** A file name for a new system that no system has. */
const freeFile = (name: string) => {
  const taken = new Set(entries.map(entry => entry.file));
  let file = `${slugOf(name)}.json`;
  for (let at = 2; taken.has(file); at++) file = `${slugOf(name)}-${at}.json`;
  return file;
};

/** A name that no system has, from the one asked for. */
const freeName = (name: string, except?: Entry) => {
  const taken = new Set(entries.filter(entry => entry !== except).map(entry => entry.saved.name));
  let free = name.trim() || 'System';
  for (let at = 2; taken.has(free); at++) free = `${name.trim() || 'System'} ${at}`;
  return free;
};

/** Writes the current system when it changed since it was last written. */
const keep = () => {
  if (!current) return;
  const text = JSON.stringify(current.saved);
  if (text === written) return;
  app.saves.write(current.file, text);
  written = text;
};

/** Reads the systems this plugin has saved, once. A blank list starts with a first system so there is something to work on. */
const load = () => {
  if (loaded) return;
  loaded = true;
  entries = app.saves
    .list()
    .filter(file => file.endsWith('.json'))
    .flatMap(file => {
      const saved = readSystem(app.saves.read(file) ?? '');
      return saved ? [{ file, saved }] : [];
    });
  if (!entries.length) add('Fire', 'Fire');
  current = entries[0];
  written = JSON.stringify(current.saved);
};

/** Makes a system from a starting point, keeps it and opens it. */
const add = (name: string, starter: string) => {
  const saved: SavedSystem = { version: 1, name: freeName(name), system: (STARTERS[starter] ?? blankSystem)() };
  const entry = { file: freeFile(saved.name), saved };
  entries.push(entry);
  app.saves.write(entry.file, JSON.stringify(saved));
  open(entry);
};

const open = (entry: Entry) => {
  keep();
  current = entry;
  editing = [];
  written = JSON.stringify(entry.saved);
};

/** The system being edited: the current one, or a sub-emitter inside it. */
const target = (): SystemDef => editing.reduce((system, at) => system.children[at].system, current!.saved.system);

/** A slider of two thumbs for a value that has a lowest and a highest. */
const rangeControl = (label: string, range: RangeDef, limits: { min: number; max: number; step: number }) => {
  inspector.slider(label, {
    ...limits,
    range: true,
    value: [finite(range.min, limits.min), finite(range.max, limits.max)],
    onChange: ([low, high]) => {
      // A number that is not a real number is not taken, so a bad value from a control cannot get into the system.
      range.min = finite(low, range.min);
      range.max = finite(high, range.max);
      invalidate();
    },
  });
};

/** A slider of one thumb. */
const numberControl = (
  label: string,
  value: number,
  limits: { min: number; max: number; step: number },
  set: (value: number) => void,
) =>
  inspector.slider(label, {
    ...limits,
    value: finite(value, limits.min, limits.min, limits.max),
    onChange: changed => {
      // A number that is not a real number is not taken, so a bad value from a control cannot get into the system.
      if (!Number.isFinite(changed)) return;
      set(changed);
      invalidate();
    },
  });

/**
 * Starts a module of the system: a foldable group, named as in Unity's particle system, with a checkbox that switches the whole module on or off.
 * Its values are kept while it is off, so they are there again when it is switched on.
 */
const moduleGroup = (system: SystemDef, name: ModuleName, label: string, then?: (on: boolean) => void) =>
  inspector.group(label, {
    enabled: moduleOn(system, name),
    onToggle: on => {
      system.modules[name] = on;
      then?.(on);
      invalidate();
    },
  });

/** The way a value is multiplied over a life, as a curve to edit, which can start from a named shape. */
const curveControl = (label: string, range: RangeDef) =>
  inspector.curve(`${label} Over Life`, {
    value: range.curve,
    min: 0,
    max: 1,
    presets: CURVE_PRESETS,
    // The particles take the curve as it is edited, and take back the old one if the editor is cancelled.
    onPreview: value => {
      range.curve = value;
      invalidate();
    },
    onChange: value => {
      range.curve = value;
    },
  });

/** What a speed counts as fast, which the three by-speed modules share, since the library has one speed range for all of them. */
const fastMeansControl = (system: SystemDef) =>
  numberControl(
    'Fast Means',
    Math.round(system.sizeBySpeed.speed * 100),
    { min: 1, max: 500, step: 1 },
    value => (system.sizeBySpeed.speed = value / 100),
  );

/** A color that is picked from a list of names and a square, which the particles take as it is dragged and give back if the picker is cancelled. */
const colorControl = (label: string, color: [number, number, number]) =>
  inspector.color(label, {
    value: color,
    swatches: SWATCH_LIST,
    onPreview: value => {
      color.splice(0, 3, value[0], value[1], value[2]);
      invalidate();
    },
    onChange: value => {
      color.splice(0, 3, value[0], value[1], value[2]);
    },
  });

/** The settings every particle starts with: its size, opacity, colors, speed and the pull on it. Always on. */
const mainControls = (system: SystemDef) => {
  const { select } = inspector;
  inspector.group('Particle System', { open: true });
  rangeControl('Start Size', system.size, { min: 0.05, max: 20, step: 0.05 });
  rangeControl('Start Opacity', system.opacity, { min: 0, max: 1, step: 0.01 });
  select('Start Colors', {
    options: ['Choose Colors', 'Picture Colors'],
    value: system.startColor ? 'Choose Colors' : 'Picture Colors',
    onChange: value => {
      system.startColor = value === 'Choose Colors' ? blankSystem().startColor : null;
    },
  });
  if (system.startColor) {
    inspector.gradient('Start Color', {
      value: system.startColor,
      swatches: SWATCH_LIST,
      // Each particle starts with a random color along the gradient. The particles take it as it is edited, and take back the old one if the editor is cancelled.
      onPreview: value => {
        system.startColor = value;
        invalidate();
      },
      onChange: value => {
        system.startColor = value;
      },
    });
  }
  rangeControl('Start Speed Sideways', system.speedX, { min: -1, max: 1, step: 0.01 });
  rangeControl('Start Speed Up And Down', system.speedY, { min: -1, max: 1, step: 0.01 });
  rangeControl('Launch Speed', system.launch.speed, { min: 0, max: 2, step: 0.01 });
  if (system.launch.speed.min > 0 || system.launch.speed.max > 0) {
    inspector.toggle('Launch Away From Center', {
      value: system.launch.outward,
      onChange: value => {
        system.launch.outward = value;
        inspector.refresh();
      },
    });
    if (!system.launch.outward) {
      numberControl(
        'Launch Direction',
        Math.round(system.launch.angle),
        { min: -180, max: 180, step: 1 },
        value => (system.launch.angle = value),
      );
    }
    numberControl(
      'Launch Spread',
      Math.round(system.launch.spread),
      { min: 0, max: 180, step: 1 },
      value => (system.launch.spread = value),
    );
  }
  numberControl(
    'Gravity',
    Math.round(system.gravity * 100),
    { min: -100, max: 100, step: 1 },
    value => (system.gravity = value / 100),
  );
  numberControl(
    'Age Spread',
    Math.round(system.ageSpread * 100),
    { min: 0, max: 100, step: 1 },
    value => (system.ageSpread = value / 100),
  );
};

/** How many particles there are: all at once, or made over time. */
const emissionControls = (system: SystemDef) => {
  // A sub-emitter is born from each particle, so it has an amount and no schedule.
  const root = editing.length === 0;
  inspector.group('Emission', {
    enabled: root ? system.emission.enabled : undefined,
    onToggle: on => {
      system.emission.enabled = on;
      invalidate();
    },
  });
  if (!root || !system.emission.enabled) {
    numberControl('Amount', system.count, { min: 1, max: 3000, step: 1 }, value => (system.count = Math.round(value)));
    return;
  }
  numberControl(
    'Emission Seconds',
    system.emission.duration,
    { min: 0.5, max: 30, step: 0.5 },
    value => (system.emission.duration = value),
  );
  inspector.toggle('Emission Repeats', {
    value: system.emission.looping,
    onChange: value => (system.emission.looping = value),
  });
  numberControl(
    'Particles Per Second',
    system.emission.rate,
    { min: 0, max: 500, step: 1 },
    value => (system.emission.rate = value),
  );
  numberControl(
    'Burst At Start',
    system.emission.burst,
    { min: 0, max: 1000, step: 1 },
    value => (system.emission.burst = Math.round(value)),
  );
  rangeControl('Lifetime In Seconds', system.emission.lifetime, { min: 0.1, max: 20, step: 0.1 });
};

/** The controls of where particles are born. */
const shapeControls = (system: SystemDef) => {
  moduleGroup(system, 'shape', 'Shape');
  // A shape with a place it comes from is moved by tapping the preview.
  inspector.select(sourceOf(system.emitter) ? 'Type (tap the preview to move it)' : 'Type', {
    options: EMITTER_TYPES.map(type => EMITTER_NAMES[type]),
    value: EMITTER_NAMES[system.emitter.type],
    onChange: name => {
      const type = EMITTER_TYPES.find(each => EMITTER_NAMES[each] === name) ?? 'image';
      system.emitter = { ...EMITTER_STARTS[type] };
    },
  });
  const percent = { min: 0, max: 100, step: 1 };
  const spot = (label: string, value: number, set: (value: number) => void, limits = percent) =>
    numberControl(label, Math.round(value * 100), limits, percentValue => set(percentValue / 100));
  const place = system.emitter;
  switch (place.type) {
    case 'circle':
      spot('Left', place.x, value => (place.x = value));
      spot('Top', place.y, value => (place.y = value));
      spot('Radius', place.radius, value => (place.radius = value), { min: 1, max: 100, step: 1 });
      break;
    case 'ring':
      spot('Left', place.x, value => (place.x = value));
      spot('Top', place.y, value => (place.y = value));
      spot('Outer Radius', place.radius, value => (place.radius = value), { min: 1, max: 100, step: 1 });
      spot('Inner Radius', place.inner, value => (place.inner = value), { min: 0, max: 100, step: 1 });
      break;
    case 'rect':
      spot('Left', place.x, value => (place.x = value));
      spot('Top', place.y, value => (place.y = value));
      spot('Width', place.width, value => (place.width = value));
      spot('Height', place.height, value => (place.height = value));
      break;
    case 'line':
      spot('Start Left', place.x1, value => (place.x1 = value));
      spot('Start Top', place.y1, value => (place.y1 = value));
      spot('End Left', place.x2, value => (place.x2 = value));
      spot('End Top', place.y2, value => (place.y2 = value));
      break;
    case 'luminance':
      spot('Lightness', place.min, value => (place.min = value));
      break;
  }
};

/** The modules that change how particles move over their life, and what speed does to them. */
const lifetimeControls = (system: SystemDef) => {
  moduleGroup(system, 'velocity', 'Velocity Over Lifetime');
  rangeControl('Spin Around Center', system.orbit, { min: -720, max: 720, step: 1 });
  rangeControl('Push From Center', system.push, { min: -1, max: 1, step: 0.01 });

  moduleGroup(system, 'limitVelocity', 'Limit Velocity Over Lifetime');
  numberControl('Drag', system.drag, { min: 0, max: 10, step: 0.5 }, value => (system.drag = value));

  moduleGroup(system, 'force', 'Force Over Lifetime');
  numberControl(
    'Wind Sideways',
    Math.round(system.windX * 100),
    { min: -100, max: 100, step: 1 },
    value => (system.windX = value / 100),
  );
  numberControl(
    'Wind Up And Down',
    Math.round(system.windY * 100),
    { min: -100, max: 100, step: 1 },
    value => (system.windY = value / 100),
  );

  moduleGroup(system, 'colorOverLifetime', 'Color Over Lifetime');
  inspector.gradient('Color', {
    value: system.lifetimeColor,
    swatches: SWATCH_LIST,
    // The particles take the gradient as it is edited, and take back the old one if the editor is cancelled.
    onPreview: value => {
      system.lifetimeColor = value;
      invalidate();
    },
    onChange: value => {
      system.lifetimeColor = value;
    },
  });

  moduleGroup(system, 'colorBySpeed', 'Color By Speed');
  colorControl('Color When Still', system.colorBySpeed.slow);
  colorControl('Color When Fast', system.colorBySpeed.fast);
  fastMeansControl(system);

  moduleGroup(system, 'sizeOverLifetime', 'Size Over Lifetime');
  curveControl('Size', system.size);

  moduleGroup(system, 'sizeBySpeed', 'Size By Speed');
  numberControl(
    'Size When Still',
    Math.round(system.sizeBySpeed.slow * 100),
    { min: 0, max: 400, step: 5 },
    value => (system.sizeBySpeed.slow = value / 100),
  );
  numberControl(
    'Size When Fast',
    Math.round(system.sizeBySpeed.fast * 100),
    { min: 0, max: 400, step: 5 },
    value => (system.sizeBySpeed.fast = value / 100),
  );
  fastMeansControl(system);

  moduleGroup(system, 'rotationOverLifetime', 'Rotation Over Lifetime');
  rangeControl('Rotation', system.rotation, { min: -360, max: 360, step: 1 });
  curveControl('Rotation', system.rotation);

  moduleGroup(system, 'rotationBySpeed', 'Rotation By Speed');
  numberControl(
    'Turn When Still',
    Math.round(system.rotationBySpeed.slow),
    { min: -720, max: 720, step: 5 },
    value => (system.rotationBySpeed.slow = value),
  );
  numberControl(
    'Turn When Fast',
    Math.round(system.rotationBySpeed.fast),
    { min: -720, max: 720, step: 5 },
    value => (system.rotationBySpeed.fast = value),
  );
  fastMeansControl(system);
};

/** Noise, collision and trails: the modules that add something more to how particles go. */
const extraControls = (system: SystemDef) => {
  moduleGroup(system, 'noise', 'Noise');
  rangeControl('Noise Strength', system.noise.strength, { min: 0, max: 0.5, step: 0.01 });
  numberControl(
    'Noise Frequency',
    system.noise.frequency,
    { min: 1, max: 60, step: 1 },
    value => (system.noise.frequency = value),
  );
  numberControl(
    'Noise Scroll Sideways',
    system.noise.scrollX,
    { min: -10, max: 10, step: 0.5 },
    value => (system.noise.scrollX = value),
  );
  numberControl(
    'Noise Scroll Up And Down',
    system.noise.scrollY,
    { min: -10, max: 10, step: 0.5 },
    value => (system.noise.scrollY = value),
  );
  numberControl(
    'Noise Octaves',
    system.noise.octaves,
    { min: 1, max: 4, step: 1 },
    value => (system.noise.octaves = Math.round(value)),
  );
  inspector.toggle('Noise Damping', { value: system.noise.damping, onChange: value => (system.noise.damping = value) });
  numberControl(
    'Noise Size Amount',
    Math.round(system.noise.sizeAmount * 100),
    { min: 0, max: 100, step: 1 },
    value => (system.noise.sizeAmount = value / 100),
  );
  numberControl(
    'Noise Rotation Amount',
    Math.round(system.noise.rotationAmount),
    { min: 0, max: 360, step: 1 },
    value => (system.noise.rotationAmount = value),
  );

  // Collision is the bounce: particles turn back at walls that are the sides of the picture, moved in by the margin.
  moduleGroup(system, 'collision', 'Collision', on => (system.bounce = on));
  numberControl(
    'Wall Margin',
    Math.round(system.bounceInset * 100),
    { min: 0, max: 40, step: 1 },
    value => (system.bounceInset = value / 100),
  );
};

/** The controls of what is drawn behind each particle. */
const trailControls = (system: SystemDef) => {
  moduleGroup(system, 'trails', 'Trails');
  numberControl(
    'Trail Length',
    Math.round(system.trail.length * 100),
    { min: 0, max: 100, step: 1 },
    value => (system.trail.length = value / 100),
  );
  numberControl(
    'Trail Fade',
    Math.round(system.trail.fade * 100),
    { min: 0, max: 100, step: 1 },
    value => (system.trail.fade = value / 100),
  );
  numberControl(
    'Trail Hardness',
    Math.round(system.trail.hardness * 100),
    { min: 0, max: 100, step: 1 },
    value => (system.trail.hardness = value / 100),
  );
};

/** How each particle is drawn. Always on. */
const rendererControls = (system: SystemDef) => {
  inspector.group('Renderer');
  inspector.select('Blend', {
    options: BLENDS,
    value: BLENDS.includes(system.blend) ? system.blend : 'Normal',
    onChange: value => {
      system.blend = value;
    },
  });
  numberControl(
    'Edge Hardness',
    Math.round(system.hardness * 100),
    { min: 0, max: 100, step: 1 },
    value => (system.hardness = value / 100),
  );
  numberControl(
    'Stretch With Speed',
    system.stretch,
    { min: 0, max: 20, step: 0.5 },
    value => (system.stretch = value),
  );
};

/** The controls of the systems born from each particle. */
const childControls = (system: SystemDef) => {
  const { select } = inspector;
  moduleGroup(system, 'subEmitters', 'Sub Emitters');
  system.children.forEach((child, at) => {
    numberControl(
      `Born At Age ${at + 1}`,
      Math.round(child.age * 100),
      { min: 0, max: 100, step: 1 },
      value => (child.age = value / 100),
    );
    select(`Sub-Emitter ${at + 1}`, {
      options: ['Edit', 'Remove'],
      momentary: true,
      onChange: value => {
        if (value === 'Edit') editing = [...editing, at];
        else system.children.splice(at, 1);
        inspector.refresh();
      },
    });
  });
  if (editing.length < MAX_CHILD_DEPTH) {
    select('Add Sub-Emitter', {
      options: Object.keys(STARTERS),
      momentary: true,
      onChange: name => {
        const child = (STARTERS[name] ?? blankSystem)();
        // A child is born at a particle, so its own place does not matter, and a few of them is plenty.
        system.children.push({
          age: 0.5,
          system: { ...child, emitter: { type: 'image' }, count: Math.min(child.count, 8) },
        });
        // Adding one is asking for them, so the module is on.
        system.modules.subEmitters = true;
      },
    });
  }
};

/** The controls for choosing, naming, making and removing the saved systems. */
const libraryControls = () => {
  const { group, select, text } = inspector;
  group('System');
  select('Saved Systems', {
    options: entries.map(entry => entry.saved.name),
    value: current!.saved.name,
    onChange: name => {
      const entry = entries.find(each => each.saved.name === name);
      if (entry) open(entry);
    },
  });
  text('Name', {
    value: current!.saved.name,
    placeholder: 'Name this system',
    onChange: value => {
      current!.saved.name = freeName(value, current!);
    },
  });
  select('New From', {
    options: Object.keys(STARTERS),
    momentary: true,
    onChange: starter => add(starter === 'Blank' ? 'New System' : starter, starter),
  });
  select('Manage', {
    options: ['Duplicate', 'Delete'],
    momentary: true,
    onChange: action => {
      const entry = current!;
      if (action === 'Duplicate') {
        const copy: SavedSystem = JSON.parse(JSON.stringify(entry.saved));
        copy.name = freeName(`${entry.saved.name} Copy`);
        const made = { file: freeFile(copy.name), saved: copy };
        entries.push(made);
        app.saves.write(made.file, JSON.stringify(copy));
        open(made);
        return;
      }
      app.saves.remove(entry.file);
      entries = entries.filter(each => each !== entry);
      current = null;
      written = '';
      if (!entries.length) add('New System', 'Blank');
      else open(entries[0]);
    },
  });

  group('Preview');
  select('Backdrop', {
    options: [NO_BACKDROP, ...Object.keys(BACKDROPS), CUSTOM_BACKDROP],
    value: backdrop,
    onChange: value => {
      backdrop = value;
    },
  });
  if (backdrop === CUSTOM_BACKDROP) {
    inspector.color('Backdrop Color', {
      value: customBackdrop,
      // The backdrop takes the color as it is dragged, and takes back the old one if the picker is cancelled.
      onPreview: value => {
        customBackdrop = [value[0], value[1], value[2]];
        invalidate();
      },
      onChange: value => {
        customBackdrop = [value[0], value[1], value[2]];
      },
    });
  }
  inspector.toggle('Play', {
    value: playing,
    onChange: value => {
      playing = value;
      startedAt = Date.now();
    },
  });
  // A playing preview goes through the life on its own, so it has a speed; one that is stopped is held at a moment, so it has an age.
  if (playing) numberControl('Cycle', cycle, { min: 0.5, max: 10, step: 0.5 }, value => (cycle = value));
  else numberControl('Age', age, { min: 0, max: 100, step: 1 }, value => (age = value));
  numberControl('Variation', variation, { min: 1, max: 999, step: 1 }, value => (variation = Math.round(value)));
};

export const inspectorGUI = () => {
  load();
  const system = target();
  if (editing.length) {
    inspector.header(`Sub-Emitter ${editing.map(at => at + 1).join('.')}`, {
      back: () => {
        editing = editing.slice(0, -1);
        inspector.refresh();
      },
    });
  } else {
    libraryControls();
  }
  mainControls(system);
  emissionControls(system);
  if (!editing.length) shapeControls(system);
  lifetimeControls(system);
  extraControls(system);
  childControls(system);
  trailControls(system);
  rendererControls(system);
};

/** How long a frame of the live preview waits for the next one, in milliseconds: about 30 frames a second. */
const FRAME_MS = 33;

/** Lets go of what was made for the system, so it is made again for the next frame. */
const dropCompiled = () => {
  compiled?.prepared.dispose();
  compiled?.made.dispose();
  compiled = null;
};

const free = () => {
  if (timer !== null) clearTimeout(timer);
  timer = null;
  dropCompiled();
  release(layer, canvas, ...backdrops.values());
  backdrops.clear();
  layer = null;
  canvas = null;
  builtFrom = '';
  picking = false;
  failing = false;
  frames = 0;
};

/** The picture the preview is drawn over: a plain color, the color the person picked, or a clear picture when there is no backdrop. */
const backdropImage = () => {
  // A picked color is a picture of its own for each color, and only the one that is showing is kept, so dragging through colors does not pile them up.
  const name = backdrop === CUSTOM_BACKDROP ? `${CUSTOM_BACKDROP}:${customBackdrop.join(',')}` : backdrop;
  let plain = backdrops.get(name);
  if (!plain) {
    if (backdrop === CUSTOM_BACKDROP) {
      for (const [other, image] of backdrops) {
        if (other.startsWith(`${CUSTOM_BACKDROP}:`)) {
          release(image);
          backdrops.delete(other);
        }
      }
    }
    const color =
      backdrop === NO_BACKDROP
        ? Color.fromRgba(0, 0, 0, 0)
        : backdrop === CUSTOM_BACKDROP
          ? Color.fromRgb(...customBackdrop)
          : Color.fromRgb(...BACKDROPS[backdrop]);
    plain = Image.newFromColor(PREVIEW[0], PREVIEW[1], color);
    release(color);
    backdrops.set(name, plain);
  }
  return plain;
};

/**
 * The system as it is now, as the effect the library draws. The particles are only worked out here: the engine draws them over the backdrop, on its
 * own thread and the GPU, each time it composes the canvas, so no pixel work is done on this thread. The caller lets the effect go.
 */
const render = (): SpritesLike | null => {
  if (!current) return null;
  // With nothing under it there are no pixels to take a color from or to find the bright parts of.
  const system = backdrop === NO_BACKDROP ? withoutPicture(current.saved.system) : current.saved.system;
  // The bright parts of a color that was picked are not those of the one before it, so a picked backdrop is part of what the system was made for.
  const key = JSON.stringify([
    system,
    variation,
    playing,
    backdrop,
    backdrop === CUSTOM_BACKDROP ? customBackdrop : null,
  ]);
  if (compiled?.key !== key) {
    dropCompiled();
    const made = toSystem(system, PREVIEW[0], PREVIEW[1], variation, 0, playing);
    compiled = { key, made, prepared: prepareParticles(made, backdropImage()) };
  }
  return particleSprites(compiled.prepared, playing ? ((Date.now() - startedAt) / (cycle * 1000)) % 1 : age / 100);
};

/** How many frames have been made, so the log can say the first few were. */
let frames = 0;

/**
 * Makes one frame of the live preview: the particles for this moment are set as the layer's live effect, which the engine draws over the backdrop
 * when it composes the canvas. Nothing is drawn here, so a frame costs only the working out of the particles.
 */
const frame = () => {
  if (!layer || !canvas) return;
  const started = Date.now();
  const batch = render();
  const rendered = Date.now();
  if (!batch) return;
  const effect = batch.liveEffect();
  layer.setLiveEffects([effect]);
  release(effect, batch);
  const handed = Date.now();
  inspector.redraw();
  if (frames++ < 20) {
    console.debug(
      `[particle maker] frame ${frames} made in ${Date.now() - started} ms: particles ${rendered - started}, live effect ${handed - rendered}, redraw ${Date.now() - handed}`,
    );
  }
  // What was changed is kept as soon as it is shown, so nothing is lost when the workspace closes.
  keep();
};

/** Whether the last frame could not be made, so a run of failures is told once and not at every frame. */
let failing = false;

/** Makes a frame, and the next one after it while the preview is playing. */
const run = () => {
  timer = null;
  let wait = FRAME_MS;
  try {
    frame();
    failing = false;
  } catch (failure) {
    if (!failing)
      console.warn('The Particle Maker could not make a frame:', String((failure as Error)?.message ?? failure));
    failing = true;
    // It tries again in a moment, because the cause can be passing (the canvas was just replaced), without filling the log.
    wait = 250;
  }
  // The play ends with the workspace, whose canvas is gone.
  if (playing && canvas) timer = setTimeout(run, wait);
};

/** Asks for a frame as soon as there can be one. A change calls it, and so does the start of a play. */
const invalidate = () => {
  if (timer === null && canvas) timer = setTimeout(run, 0);
};

/** The name of the dot on the preview that marks where the particles come from. */
const SOURCE = 'source';

/** Lets a tap on the preview move where the particles come from, and puts the dot where they come from now. */
const showSource = (shown: CanvasLike) => {
  if (!picking) {
    picking = true;
    const color = Color.fromRgb(255, 170, 40);
    app.gizmos.color(color);
    release(color);
    app.gizmos.pickPoint(SOURCE, point => {
      // A tap changes the system being edited only when it is the system itself, which is the one with a place on the preview.
      if (
        editing.length ||
        !current ||
        !moduleOn(current.saved.system, 'shape') ||
        !sourceOf(current.saved.system.emitter)
      )
        return;
      moveSource(current.saved.system.emitter, point);
      inspector.refresh();
    });
  }
  const source = current && moduleOn(current.saved.system, 'shape') && sourceOf(current.saved.system.emitter);
  // The preview shows the canvas it is given after this runs, so the dot is placed by the size of that one.
  if (source) app.gizmos.setPoint(SOURCE, source, shown);
  else app.gizmos.remove(SOURCE);
};

export const previewCanvas = () => {
  load();
  if (!current) return null;
  // The canvas is made again only for another backdrop. The system itself is drawn onto its layer by the live preview, as it changes.
  if (!canvas || builtFrom !== backdrop) {
    const next = Canvas.newBlank('Particles', PREVIEW[0], PREVIEW[1]);
    const nextLayer = next.addLayerFromImage('Backdrop', backdropImage(), undefined);
    release(layer, canvas);
    canvas = next;
    layer = nextLayer;
    builtFrom = backdrop;
    frames = 0;
    // The system is on the layer before the preview shows the canvas, so what it first shows is the particles and not the bare backdrop.
    frame();
    console.debug(`[particle maker] canvas made, ${playing ? 'playing' : 'paused'}`);
  }
  showSource(canvas);
  invalidate();
  return canvas;
};

/** The workspace closed: what was made is kept, and what was held is let go. */
export const dispose = () => {
  keep();
  free();
};
