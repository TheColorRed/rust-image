import { BOKEH_SHAPES, MAX_ORBS, RANDOM_ROTATION, bokeh, type OrbKind } from '@/src/effects/bokeh';
import { MixedControl, SliderControl } from '@/src/lib/project/edit-sections';

/** The look of one kind of orb from its slider's values: color, fewest and most orbs, rotation, strength, then the saved shape. */
const orbKind = ([hue, minOrbs, maxOrbs, rotation, strength, shape]: number[]): OrbKind => ({
  hue,
  minOrbs,
  maxOrbs,
  rotation,
  strength,
  shape: BOKEH_SHAPES[shape] ?? 'circle',
});

export const bokehControl: MixedControl = {
  kind: 'mixed',
  key: 'action-bokeh',
  label: 'Bokeh',
  thumb: photo => bokeh(photo, 0, [orbKind([0.1, 3, 7, 0, 1, 0]), orbKind([0.9, 3, 7, 0, 1, 0])]),
  // The values of each slider, in order: the layout, then each orb's color, fewest and most orbs, rotation, strength, and
  // the shape its picker saved, as a place in `BOKEH_SHAPES` (the circle until one is chosen).
  tool: (photo, [[layout], ...orbs]) => bokeh(photo, layout, orbs.map(orbKind)),
  controls: [
    {
      kind: 'slider',
      key: 'action-bokeh-layout',
      label: 'Layout',
      min: 0,
      max: 99,
      step: 1,
      defaultValue: 0,
      applyOnSelect: true,
    } satisfies SliderControl<number>,
    ...[1, 2].map(
      (orb): MixedControl => ({
        kind: 'mixed',
        key: `action-bokeh-orb-${orb}`,
        label: `Orb ${orb}`,
        controls: [
          {
            kind: 'slider',
            key: `action-bokeh-orb-${orb}-settings`,
            label: 'Settings',
            sliderLabel: ['Color', 'Orbs', 'Rotation', 'Strength'],
            display: 'tabulated',
            pillFormat: [undefined, 'plain', 'plain', undefined],
            valueText: [
              undefined,
              undefined,
              value => (value >= RANDOM_ROTATION ? 'Random' : undefined),
              undefined,
            ],
            picker: ['hue', undefined, undefined, undefined],
            min: [0, [0, 0], 0, 0],
            max: [1, [MAX_ORBS, MAX_ORBS], RANDOM_ROTATION, 1],
            step: [0.01, 1, 1, 0.01],
            defaultValue: [orb === 1 ? 0.1 : 0.9, [3, 7], RANDOM_ROTATION, 1],
            applyOnSelect: true,
          } satisfies SliderControl<number>,
          {
            kind: 'picker',
            key: `action-bokeh-shape-${orb}`,
            label: 'Shape',
            selection: 'one',
            commitTo: { slider: `action-bokeh-orb-${orb}-settings`, index: 5 },
            after: 0,
            items: BOKEH_SHAPES.map((shape, value) => ({
              key: `bokeh-shape-${orb}-${shape}`,
              label:
                shape === 'random'
                  ? 'Random'
                  : shape.replace('-', ' ').replace(/^./, letter => letter.toUpperCase()),
              picture: { kind: 'rendered' as const },
              value,
            })),
          },
        ],
      }),
    ),
  ],
};
