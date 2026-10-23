/**
 * Puts `controls` into a list of controls (a section's, or a group's), after the control with the key `after` in it, or at the end when
 * that key is not there or none is given. Changes `list` in place, because the editor reads the same arrays everywhere.
 */
export function insertControls<T extends { key: string }>(list: T[], controls: readonly T[], after?: string): void {
  const at = after === undefined ? -1 : list.findIndex(control => control.key === after);
  if (at < 0) list.push(...controls);
  else list.splice(at + 1, 0, ...controls);
}

/** Takes `controls` back out of a list, wherever they ended up in it. Changes `list` in place. */
export function removeControls<T>(list: T[], controls: readonly T[]): void {
  for (const control of controls) {
    const at = list.indexOf(control);
    if (at >= 0) list.splice(at, 1);
  }
}
