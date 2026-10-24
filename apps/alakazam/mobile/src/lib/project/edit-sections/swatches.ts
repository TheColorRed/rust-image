/**
 * Makes a control's swatches from a list of colors. `values` lays the color out the way the control's sliders do, from the
 * color's hue (turned the way the color bar runs), vividness and brightness.
 */
export const swatchesOf = (
  colors: { label: string; color: string }[],
  values: (hue: number, saturation: number, brightness: number, label: string) => (number | undefined)[],
) =>
  colors.map(({ label, color }) => {
    const [r, g, b] = [1, 3, 5].map(at => parseInt(color.slice(at, at + 2), 16) / 255);
    const high = Math.max(r, g, b);
    const range = high - Math.min(r, g, b);
    const degrees = !range
      ? 0
      : high === r
        ? ((g - b) / range + 6) % 6
        : high === g
          ? (b - r) / range + 2
          : (r - g) / range + 4;
    return { label, color, values: values((1 - (degrees * 60) / 360) % 1, high ? range / high : 0, high, label) };
  });
