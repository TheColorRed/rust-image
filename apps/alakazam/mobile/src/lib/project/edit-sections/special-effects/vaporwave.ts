import { vaporwave } from '@/src/effects/vaporwave';
import { MixedControl, SliderControl } from '@/src/lib/project/edit-sections';
import { NEON_COLORS, colorAlongBar } from '@/src/lib/ui/color-bars';

/**
 * The look from the control's saved values: the grid color (a place on the neon bar), the strength, then the sun and the grid as
 * the places their pickers saved: 0 draws the part (what it does until one is chosen) and 1 leaves it out.
 */
const look = ([color = 0, , sun, grid]: number[]) => ({
  sun: sun !== 1,
  grid: grid !== 1,
  gridColor: colorAlongBar(NEON_COLORS, color) as [number, number, number],
});

export const vaporwaveControl: MixedControl = {
  kind: 'mixed',
  key: 'action-vaporwave',
  label: 'Vaporwave',
  thumb: photo => vaporwave(photo, 1, look([])),
  tool: (photo, [values]) => vaporwave(photo, values[1] ?? 1, look(values)),
  controls: [
    {
      kind: 'slider',
      key: 'action-vaporwave-settings',
      label: 'Settings',
      sliderLabel: ['Grid color', 'Strength'],
      // Tabulated, so the Sun and Grid pickers show as tabs beside the sliders.
      display: 'tabulated',
      pillFormat: [undefined, 'percent'],
      picker: ['neon', undefined],
      min: [0, 0],
      max: [1, 1],
      step: [0.01, 0.01],
      defaultValue: [0, 1],
      applyOnSelect: true,
    } satisfies SliderControl<number>,
    ...(['sun', 'grid'] as const).map(
      (part, index): MixedControl['controls'][number] => ({
        kind: 'picker',
        key: `action-vaporwave-${part}`,
        label: part === 'sun' ? 'Sun' : 'Grid',
        selection: 'one',
        commitTo: { slider: 'action-vaporwave-settings', index: index + 2 },
        after: -1,
        items: [0, 1].map(value => ({
          key: `vaporwave-${part}-${value}`,
          label: `${value === 0 ? 'With' : 'Without'} ${part}`,
          picture: { kind: 'rendered' as const },
          value,
          // Only this part changes: the other parts are as the other tabs have them now.
          thumb: (photo, { values = [] }) => vaporwave(photo, 1, { ...look(values), [part]: value === 0 }),
        })),
      }),
    ),
  ],
};
