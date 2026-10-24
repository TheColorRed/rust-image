/**
 * What the editor needs to know about a control without holding the control: whether it can be set for one person, and whether its work
 * is confined to a face. The controls say so themselves (`person` and `face` on a slider), and the list is filled from them when they
 * are made, and when a plugin adds some, so nothing here names a control. This module imports no controls, because the state is read
 * before and without them.
 */
const people = new Set<string>();
const faces = new Set<string>();

/** Whether the control with this key can be set for one person, or for everyone. */
export const isPersonControlKey = (key: string): boolean => people.has(key);

/** Whether the work of the control with this key is confined to a face, so opening it can bring that face closer. */
export const isFaceControlKey = (key: string): boolean => faces.has(key);

/** Notes what a control says about itself. Calling it again for the same key replaces what was noted. */
export const setControlTraits = (key: string, traits: { person?: boolean; face?: boolean }): void => {
  if (traits.person) people.add(key);
  else people.delete(key);
  if (traits.face) faces.add(key);
  else faces.delete(key);
};

/** Forgets a control, such as one that a plugin added and that was taken away. */
export const clearControlTraits = (key: string): void => {
  people.delete(key);
  faces.delete(key);
};
